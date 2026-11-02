//! iOS native LUT preview worker. One in-flight frame; UI sees only status.
use crate::preview_histogram::Histogram;
use crate::{imaging_document::ImagingDocument, photo_look::PhotoRenderer};
use camera_apple::{AppleCaptureSession, IosPreviewHost};
use imaging_gpu::{
    native_input::NativeTextureImporter,
    preview::{NativeSurface, PeakingColor, PreviewFrame, PreviewPipeline},
    wgpu,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Default)]
pub struct Status {
    pub running: AtomicBool,
    pub enabled: AtomicBool,
    pub ready: AtomicBool,
    pub zebra: AtomicBool,
    pub presented_zebra: AtomicBool,
    pub focus: AtomicBool,
    pub presented_focus: AtomicBool,
    pub presented_revision: AtomicU64,
    pub stop: AtomicBool,
    pub presented: AtomicU64,
    pub histogram: Mutex<Option<Histogram>>,
    pub histogram_updates: AtomicU64,
    pub error: Mutex<Option<String>>,
    pub last_request: Mutex<Option<std::time::Instant>>,
    visibility_target: AtomicBool,
}
impl Status {
    pub fn request(&self, enabled: bool) {
        if let Ok(mut last) = self.last_request.lock() {
            *last = Some(std::time::Instant::now());
        }
        self.enabled.store(enabled, Ordering::Release);
    }
    fn active(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
            && self
                .last_request
                .lock()
                .ok()
                .and_then(|last| *last)
                .is_some_and(|time| time.elapsed() < std::time::Duration::from_secs(1))
    }
}
pub struct Worker {
    pub status: Arc<Status>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start(
        window: tauri::WebviewWindow,
        host: IosPreviewHost,
        session: Arc<AppleCaptureSession>,
        renderer: Arc<PhotoRenderer>,
        document: Arc<Mutex<Option<ImagingDocument>>>,
        luts: Result<std::path::PathBuf, String>,
    ) -> Self {
        let status = Arc::new(Status::default());
        let shared = status.clone();
        shared.running.store(true, Ordering::Release);
        let thread = std::thread::Builder::new()
            .name("native-lut-preview".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match luts {
                        Ok(luts) => run(
                            &window, host, &session, &renderer, &document, &luts, &shared,
                        ),
                        Err(error) => Err(error),
                    }))
                    .unwrap_or_else(|_| Err("native preview worker failed".into()));
                shared.running.store(false, Ordering::Release);
                if let Err(error) = result {
                    if let Ok(mut value) = shared.error.lock() {
                        *value = Some(error);
                    }
                }
                visibility(&window, host, &shared, false);
            });
        let thread = match thread {
            Ok(thread) => Some(thread),
            Err(error) => {
                status.running.store(false, Ordering::Release);
                if let Ok(mut e) = status.error.lock() {
                    *e = Some(error.to_string());
                }
                None
            }
        };
        Self { status, thread }
    }
    pub fn stop(mut self) {
        self.status.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.status.stop.store(true, Ordering::Release);
    }
}

fn visibility(
    window: &tauri::WebviewWindow,
    host: IosPreviewHost,
    status: &Arc<Status>,
    visible: bool,
) {
    if !visible {
        status.ready.store(false, Ordering::Release);
    }
    // Coalesce UIKit transitions while the main thread is busy; rendering must
    // not create one queued UI closure per camera frame.
    if status.visibility_target.swap(visible, Ordering::AcqRel) == visible {
        return;
    }
    let shared = status.clone();
    let _ = window.run_on_main_thread(move || {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let show = visible && shared.running.load(Ordering::Acquire) && shared.active();
        if host.set_gpu_visible(show).is_ok() {
            shared.ready.store(show, Ordering::Release);
        }
    });
}

fn run(
    window: &tauri::WebviewWindow,
    host: IosPreviewHost,
    session: &AppleCaptureSession,
    renderer: &PhotoRenderer,
    document: &Mutex<Option<ImagingDocument>>,
    luts: &std::path::Path,
    status: &Arc<Status>,
) -> Result<(), String> {
    while !status.active() && !status.stop.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    if status.stop.load(Ordering::Acquire) {
        return Ok(());
    }
    // Device and pipeline initialization occurs off the UI thread, shared with stills.
    let (instance, adapter, device, queue) = renderer.preview_context()?;
    // SAFETY: stop joins this worker before the host releases its retained layer.
    let mut surface =
        unsafe { NativeSurface::new(&instance, &adapter, &device, host.metal_layer) }?;
    let importer = NativeTextureImporter::new(&device)?;
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut pipeline = PreviewPipeline::new(&device, surface.config.format);
    if let Some(error) = pollster_error(&device) {
        return Err(error);
    }
    let mut revision = None;
    let mut rejected_revision = None;
    let mut lut = None;
    let mut was_enabled = false;
    let mut last_histogram: Option<(u64, std::time::Instant)> = None;
    while !status.stop.load(Ordering::Acquire) {
        if !status.active() {
            if was_enabled {
                visibility(window, host, status, false);
                was_enabled = false;
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
            continue;
        }
        was_enabled = true;
        let snapshot = document
            .lock()
            .map_err(|_| "imaging document lock poisoned")?
            .clone()
            .ok_or("imaging document unavailable")?;
        if !snapshot.monitoring.as_ref().is_some_and(|m| {
            m.color_space != "display-p3"
                && (m.look_enabled
                    || status.focus.load(Ordering::Acquire)
                    || status.zebra.load(Ordering::Acquire))
        }) {
            if status.ready.load(Ordering::Acquire) {
                visibility(window, host, status, false);
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
            continue;
        }
        if revision != Some(snapshot.revision) {
            if rejected_revision == Some(snapshot.revision) {
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
            let selection = if snapshot.monitoring.as_ref().unwrap().look_enabled {
                &snapshot.selected_lut
            } else {
                "none"
            };
            let payload = match crate::lut_catalog::payload(luts, selection) {
                Ok(payload) => payload,
                Err(error) => {
                    rejected_revision = Some(snapshot.revision);
                    if let Ok(mut e) = status.error.lock() {
                        *e = Some(error);
                    }
                    visibility(window, host, status, false);
                    continue;
                }
            };
            lut = Some(imaging_gpu::Lut3d::new(
                payload.size,
                &payload.samples,
                payload.domain_min,
                payload.domain_max,
            )?);
            revision = Some(snapshot.revision);
        }
        let Some(frame) = session.native_frames().take_latest() else {
            std::thread::sleep(std::time::Duration::from_millis(3));
            continue;
        };
        let color = match frame.preview_color() {
            Ok(color) => color,
            Err(error) => {
                if let Ok(mut e) = status.error.lock() {
                    *e = Some(error);
                }
                if status.ready.load(Ordering::Acquire) {
                    visibility(window, host, status, false);
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
        };
        let orientation = session.capture_orientation();
        let layer = unsafe { &*(host.metal_layer as *const objc2_quartz_core::CAMetalLayer) };
        let bounds = layer.bounds();
        let scale = layer.contentsScale();
        let extent = [
            (bounds.size.width * scale).round() as u32,
            (bounds.size.height * scale).round() as u32,
        ];
        if extent.contains(&0) {
            continue;
        }
        // SAFETY: move the entire lease into the queue-completion callback below.
        let input = match unsafe { importer.import(frame) } {
            Ok(input) => input,
            Err(error) => {
                if let Ok(mut e) = status.error.lock() {
                    *e = Some(error);
                }
                visibility(window, host, status, false);
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
        };
        let target = match surface.acquire(&device, extent) {
            Ok(target) => target,
            Err(error) => {
                visibility(window, host, status, false);
                if let Ok(mut e) = status.error.lock() {
                    *e = Some(error);
                }
                continue;
            }
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let zebra = status.zebra.load(Ordering::Acquire);
        let focus = status.focus.load(Ordering::Acquire);
        let collect_histogram = last_histogram.is_none_or(|(revision, time)| {
            revision != snapshot.revision || time.elapsed() >= std::time::Duration::from_millis(100)
        });
        let peaking = if focus {
            Some(
                PeakingColor::from_name(&snapshot.monitoring.as_ref().unwrap().peaking_color)
                    .ok_or("unsupported peaking color")?,
            )
        } else {
            None
        };
        let encoded = pipeline.encode(
            &device,
            &queue,
            &mut encoder,
            PreviewFrame {
                input: &input.planes,
                target: &target.texture.create_view(&Default::default()),
                extent,
                color,
                lut: lut.as_ref().unwrap(),
                zebra,
                peaking,
                apply_lut: snapshot.monitoring.as_ref().unwrap().look_enabled,
                collect_histogram,
            },
        );
        if let Err(error) = encoded {
            let _ = pollster_error(&device);
            return Err(error);
        }
        let submission = queue.submit([encoder.finish()]);
        queue.on_submitted_work_done(move || drop(input));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(2)),
            })
            .map_err(|e| e.to_string())?;
        if let Some(error) = pollster_error(&device) {
            return Err(error);
        }
        let current = session.capture_orientation();
        if !status.active()
            || current.rotation_degrees != orientation.rotation_degrees
            || current.preview_mirrored != orientation.preview_mirrored
        {
            continue;
        }
        target.present();
        if collect_histogram {
            let bins = pipeline.read_histogram(&device)?;
            let collected = std::time::Instant::now();
            if let Ok(mut histogram) = status.histogram.lock() {
                *histogram = Some(Histogram::new(snapshot.revision, collected, bins));
                status.histogram_updates.fetch_add(1, Ordering::Relaxed);
            }
            last_histogram = Some((snapshot.revision, collected));
        }
        status.presented_zebra.store(zebra, Ordering::Release);
        status.presented_focus.store(focus, Ordering::Release);
        status
            .presented_revision
            .store(snapshot.revision, Ordering::Release);
        status.presented.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut e) = status.error.lock() {
            *e = None;
        }
        if !status.ready.load(Ordering::Acquire) {
            visibility(window, host, status, true);
        }
    }
    Ok(())
}

fn pollster_error(device: &wgpu::Device) -> Option<String> {
    tauri::async_runtime::block_on(device.pop_error_scope()).map(|e| e.to_string())
}
