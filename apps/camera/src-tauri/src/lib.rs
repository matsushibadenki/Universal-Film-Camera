#[cfg(target_os = "android")]
mod android_camera;
#[cfg(target_os = "macos")]
mod camera_settings;
mod nearby_discovery;

#[cfg(not(target_os = "android"))]
use camera_apple::AppleCameraBackend;
#[cfg(target_os = "ios")]
use camera_apple::IosPreviewHost as PlatformPreviewHost;
#[cfg(target_os = "macos")]
use camera_apple::MacPreviewHost as PlatformPreviewHost;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use camera_apple::{ActiveCameraFormat, AppleCaptureSession, PreviewRect};
#[cfg(not(target_os = "android"))]
use camera_core::CameraBackend;
use camera_core::{
    CameraAuthorizationStatus, CameraCapabilities, CameraController, CameraDevice, CameraMode,
    CameraState, CaptureOrientation, CapturedAsset, MediaIndex, MediaIndexEntry,
};
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
use camera_core::{
    CaptureMetadata, CapturedMediaType, RationalRate, SelectedCaptureFormat, probe_media_resource,
};
#[cfg(target_os = "macos")]
use camera_settings::{StoredCameraFormat, load_format, save_format};
use imaging_core::SignalDomain;
use nearby_discovery::{
    NearbyDiscoverySnapshot, NearbyDiscoveryState, StartNearbyDiscoveryRequest,
};
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "android"))]
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

struct AppState {
    camera: Mutex<CameraController>,
    #[cfg(not(target_os = "android"))]
    backend: Arc<dyn CameraBackend>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    preview: Mutex<Option<PreviewRuntime>>,
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
    pending_movie: Mutex<Option<PendingMovie>>,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct PreviewRuntime {
    session: Arc<AppleCaptureSession>,
    host: PlatformPreviewHost,
    device_id: String,
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
struct PendingMovie {
    id: String,
    temporary: std::path::PathBuf,
    destination: std::path::PathBuf,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    capture: CaptureMetadata,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            camera: Mutex::new(CameraController::default()),
            #[cfg(not(target_os = "android"))]
            backend: Arc::new(AppleCameraBackend),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            preview: Mutex::new(None),
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
            pending_movie: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
struct CameraStatus {
    state: CameraState,
    mode: CameraMode,
}

#[derive(Serialize)]
struct CameraDiscovery {
    authorization: CameraAuthorizationStatus,
    devices: Vec<CameraDevice>,
}

#[derive(Serialize)]
struct ImagingPipelineContract {
    schema_version: u32,
    domains: Vec<SignalDomain>,
}

#[derive(Serialize)]
struct CaptureOutputPreset {
    id: &'static str,
    media_type: &'static str,
    container: &'static str,
    video_codec: Option<&'static str>,
    audio_codec: Option<&'static str>,
    estimated_bytes_per_unit: u64,
}

#[derive(Serialize)]
struct CaptureOutputPresets {
    still: Vec<CaptureOutputPreset>,
    video: Vec<CaptureOutputPreset>,
}

#[derive(Serialize)]
struct CaptureStorageStatus {
    path: std::path::PathBuf,
    available_bytes: u64,
    total_bytes: u64,
    photo_ready: bool,
    video_ready: bool,
}

const PHOTO_ESTIMATED_BYTES: u64 = 8 * 1024 * 1024;
const VIDEO_MINIMUM_ESTIMATED_BYTES: u64 = 120 * 1024 * 1024;
const CAPTURE_STORAGE_RESERVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
struct PreviewViewport {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
struct PreviewStatus {
    running: bool,
    device_id: String,
    active_format: Option<PreviewFormat>,
    format_restored: bool,
    settings_warning: Option<String>,
    orientation: CaptureOrientation,
}

#[derive(Serialize)]
struct PreviewFormat {
    width: u32,
    height: u32,
    fps: f64,
    settings_persisted: bool,
    settings_warning: Option<String>,
}

#[cfg(target_os = "android")]
impl From<android_camera::PreviewFormatResponse> for PreviewFormat {
    fn from(value: android_camera::PreviewFormatResponse) -> Self {
        Self {
            width: value.width,
            height: value.height,
            fps: value.fps,
            settings_persisted: value.settings_persisted,
            settings_warning: value.settings_warning,
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl From<PreviewViewport> for PreviewRect {
    fn from(value: PreviewViewport) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl From<ActiveCameraFormat> for PreviewFormat {
    fn from(value: ActiveCameraFormat) -> Self {
        Self {
            width: value.width,
            height: value.height,
            fps: value.fps,
            settings_persisted: true,
            settings_warning: None,
        }
    }
}

#[cfg(target_os = "macos")]
fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?
        .join("settings")
        .join("camera-format-v1.json"))
}

fn captures_directory(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?
        .join("captures"))
}

#[tauri::command]
fn get_capture_output_presets() -> CaptureOutputPresets {
    CaptureOutputPresets {
        still: vec![CaptureOutputPreset {
            id: "jpeg_high",
            media_type: "photo",
            container: "jpeg",
            video_codec: None,
            audio_codec: None,
            estimated_bytes_per_unit: PHOTO_ESTIMATED_BYTES,
        }],
        video: vec![CaptureOutputPreset {
            id: "h264_aac_balanced",
            media_type: "video",
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            container: "quicktime",
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            container: "mp4",
            video_codec: Some("h264"),
            audio_codec: Some("aac"),
            estimated_bytes_per_unit: VIDEO_MINIMUM_ESTIMATED_BYTES,
        }],
    }
}

#[cfg(unix)]
fn filesystem_capacity(path: &std::path::Path) -> Result<(u64, u64), String> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let encoded = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "capture storage path contains an interior NUL byte".to_string())?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `encoded` is NUL-terminated and `stats` points to writable storage.
    let result = unsafe { libc::statvfs(encoded.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(format!(
            "failed to inspect capture storage: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: statvfs returned success and initialized the output structure.
    let stats = unsafe { stats.assume_init() };
    let block_size = stats.f_frsize as u64;
    Ok((
        (stats.f_bavail as u64).saturating_mul(block_size),
        (stats.f_blocks as u64).saturating_mul(block_size),
    ))
}

#[tauri::command]
fn get_capture_storage_status(app: tauri::AppHandle) -> Result<CaptureStorageStatus, String> {
    let path = captures_directory(&app)?;
    std::fs::create_dir_all(&path)
        .map_err(|error| format!("failed to create capture storage: {error}"))?;
    #[cfg(unix)]
    let (available_bytes, total_bytes) = filesystem_capacity(&path)?;
    #[cfg(not(unix))]
    let (available_bytes, total_bytes) = (0, 0);
    #[cfg(unix)]
    let (photo_ready, video_ready) = (
        capture_capacity_ready(available_bytes, PHOTO_ESTIMATED_BYTES),
        capture_capacity_ready(available_bytes, VIDEO_MINIMUM_ESTIMATED_BYTES),
    );
    #[cfg(not(unix))]
    let (photo_ready, video_ready) = (true, true);
    Ok(CaptureStorageStatus {
        path,
        available_bytes,
        total_bytes,
        photo_ready,
        video_ready,
    })
}

fn capture_capacity_ready(available_bytes: u64, estimated_output_bytes: u64) -> bool {
    available_bytes >= CAPTURE_STORAGE_RESERVE_BYTES.saturating_add(estimated_output_bytes)
}

fn ensure_capture_capacity(
    path: &std::path::Path,
    estimated_output_bytes: u64,
) -> Result<(), String> {
    #[cfg(unix)]
    let (available_bytes, _) = filesystem_capacity(path)?;
    #[cfg(not(unix))]
    let available_bytes = u64::MAX;
    if !capture_capacity_ready(available_bytes, estimated_output_bytes) {
        return Err(format!(
            "capture storage is too low: {} bytes available; {} bytes required including safety reserve",
            available_bytes,
            CAPTURE_STORAGE_RESERVE_BYTES.saturating_add(estimated_output_bytes)
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn start_apple_storage_monitor(
    app: tauri::AppHandle,
    session: Arc<AppleCaptureSession>,
    pending_id: String,
    captures: std::path::PathBuf,
) {
    tauri::async_runtime::spawn_blocking(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let state = app.state::<AppState>();
            let is_current_recording = state
                .pending_movie
                .lock()
                .map(|pending| pending.as_ref().is_some_and(|movie| movie.id == pending_id))
                .unwrap_or(false)
                && state
                    .camera
                    .lock()
                    .map(|camera| camera.state() == CameraState::Recording)
                    .unwrap_or(false);
            if !is_current_recording {
                break;
            }
            let Ok((available_bytes, _)) = filesystem_capacity(&captures) else {
                continue;
            };
            if !capture_capacity_ready(available_bytes, VIDEO_MINIMUM_ESTIMATED_BYTES) {
                let _ = session.request_recording_stop();
                break;
            }
        }
    });
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
fn finalize_captured_asset(
    temporary: &std::path::Path,
    destination: &std::path::Path,
    asset: &CapturedAsset,
) -> Result<(), String> {
    std::fs::rename(temporary, destination)
        .map_err(|error| format!("failed to finalize validated media: {error}"))?;
    let captures = destination
        .parent()
        .ok_or("finalized media path has no captures directory")?;
    if let Err(error) = MediaIndex::new(captures).persist_finalized(asset) {
        let rollback = std::fs::rename(destination, temporary);
        return Err(match rollback {
            Ok(()) => format!(
                "failed to persist media manifest; resource was returned to incomplete storage: {error}"
            ),
            Err(rollback_error) => format!(
                "failed to persist media manifest ({error}) and failed to roll resource back ({rollback_error})"
            ),
        });
    }
    Ok(())
}

#[tauri::command]
fn get_media_index(app: tauri::AppHandle) -> Result<Vec<MediaIndexEntry>, String> {
    MediaIndex::new(captures_directory(&app)?)
        .list()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reconcile_media_index(app: tauri::AppHandle) -> Result<Vec<MediaIndexEntry>, String> {
    MediaIndex::new(captures_directory(&app)?)
        .reconcile_orphans()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cleanup_media_entry(app: tauri::AppHandle, id: String) -> Result<Vec<MediaIndexEntry>, String> {
    let index = MediaIndex::new(captures_directory(&app)?);
    index
        .cleanup_recoverable(&id)
        .map_err(|error| error.to_string())?;
    index.list().map_err(|error| error.to_string())
}

#[tauri::command]
fn reinspect_media_entry(
    app: tauri::AppHandle,
    id: String,
) -> Result<Vec<MediaIndexEntry>, String> {
    MediaIndex::new(captures_directory(&app)?)
        .reinspect_recoverable(&id)
        .map_err(|error| error.to_string())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn selected_capture_format(format: ActiveCameraFormat) -> SelectedCaptureFormat {
    const RATE_SCALE: u64 = 1_000_000;
    let numerator = (format.fps * RATE_SCALE as f64).round().max(1.0) as u64;
    SelectedCaptureFormat {
        width: format.width,
        height: format.height,
        fps: RationalRate {
            numerator,
            denominator: RATE_SCALE,
        },
    }
}

#[cfg(target_os = "android")]
fn capture_metadata_from_resource(
    device_id: &str,
    resource: &camera_core::MediaResource,
    requested: Option<android_camera::PreviewFormatResponse>,
) -> CaptureMetadata {
    const RATE_SCALE: u64 = 1_000_000;
    let selected_format = requested.map_or_else(
        || SelectedCaptureFormat {
            width: resource.pixel_width,
            height: resource.pixel_height,
            fps: resource.frame_rate.clone().unwrap_or(RationalRate {
                numerator: 30,
                denominator: 1,
            }),
        },
        |format| SelectedCaptureFormat {
            width: format.width,
            height: format.height,
            fps: RationalRate {
                numerator: (format.fps * RATE_SCALE as f64).round().max(1.0) as u64,
                denominator: RATE_SCALE,
            },
        },
    );
    CaptureMetadata {
        device_id: device_id.to_owned(),
        selected_format,
    }
}

#[tauri::command]
fn get_camera_status(state: tauri::State<'_, AppState>) -> Result<CameraStatus, String> {
    let camera = state
        .camera
        .lock()
        .map_err(|_| "camera state lock poisoned")?;
    Ok(CameraStatus {
        state: camera.state(),
        mode: camera.mode(),
    })
}

#[cfg(not(target_os = "android"))]
fn discovery(backend: &dyn CameraBackend) -> Result<CameraDiscovery, String> {
    let authorization = backend.authorization_status();
    let devices = backend.devices().map_err(|error| error.to_string())?;
    Ok(CameraDiscovery {
        authorization,
        devices,
    })
}

#[tauri::command]
fn get_camera_discovery(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CameraDiscovery, String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        let result = android_camera::discovery(&app)?;
        return Ok(CameraDiscovery {
            authorization: result.authorization,
            devices: result.devices,
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        discovery(state.backend.as_ref())
    }
}

#[tauri::command]
fn get_camera_capabilities(
    device_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CameraCapabilities, String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        return android_camera::capabilities(&app, &device_id);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        state
            .backend
            .capabilities(&device_id)
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
async fn apply_camera_format(
    width: u32,
    height: u32,
    fps: u32,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PreviewFormat, String> {
    #[cfg(target_os = "android")]
    {
        if state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .state()
            != CameraState::Previewing
        {
            return Err("camera format can only change while previewing".into());
        }
        let active = android_camera::apply_format(&app, width, height, fps).await?;
        return Ok(active.into());
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .state()
            != CameraState::Previewing
        {
            return Err("camera format can only change while previewing".into());
        }
        let (session, device_id) = {
            let preview = state
                .preview
                .lock()
                .map_err(|_| "preview state lock poisoned")?;
            let runtime = preview.as_ref().ok_or("camera preview is not running")?;
            (Arc::clone(&runtime.session), runtime.device_id.clone())
        };
        let active = tauri::async_runtime::spawn_blocking(move || {
            session.set_active_format(width, height, fps)
        })
        .await
        .map_err(|error| format!("camera format task failed: {error}"))?
        .map_err(|error| error.to_string())?;
        let mut response: PreviewFormat = active.into();
        #[cfg(target_os = "macos")]
        {
            let settings_result = settings_path(&app).and_then(|path| {
                save_format(&path, &device_id, StoredCameraFormat { width, height, fps })
            });
            response.settings_persisted = settings_result.is_ok();
            response.settings_warning = settings_result.err();
        }
        #[cfg(target_os = "ios")]
        {
            let _ = (app, device_id);
            response.settings_persisted = false;
        }
        return Ok(response);
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (width, height, fps, app, state);
        Err("camera format selection is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn request_camera_authorization(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CameraDiscovery, String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        let result = android_camera::request_authorization(&app).await?;
        return Ok(CameraDiscovery {
            authorization: result.authorization,
            devices: result.devices,
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let backend = Arc::clone(&state.backend);
        tauri::async_runtime::spawn_blocking(move || {
            backend
                .request_authorization()
                .map_err(|error| error.to_string())?;
            discovery(backend.as_ref())
        })
        .await
        .map_err(|error| format!("camera authorization task failed: {error}"))?
    }
}

#[tauri::command]
fn get_microphone_authorization(app: tauri::AppHandle) -> CameraAuthorizationStatus {
    #[cfg(target_os = "android")]
    {
        return android_camera::microphone_authorization(&app)
            .unwrap_or(CameraAuthorizationStatus::Unavailable);
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        return AppleCameraBackend.microphone_authorization_status();
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    CameraAuthorizationStatus::Unavailable
}

#[tauri::command]
async fn request_microphone_authorization(
    app: tauri::AppHandle,
) -> Result<CameraAuthorizationStatus, String> {
    #[cfg(target_os = "android")]
    {
        return android_camera::request_microphone_authorization(&app).await;
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        return tauri::async_runtime::spawn_blocking(|| {
            AppleCameraBackend
                .request_microphone_authorization()
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| format!("microphone authorization task failed: {error}"))?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    Err("microphone authorization is unavailable on this platform".into())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn attach_preview_host(
    window: &tauri::WebviewWindow,
    session: Arc<AppleCaptureSession>,
    viewport: PreviewViewport,
) -> Result<PlatformPreviewHost, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    window
        .with_webview(move |webview| {
            // Attach to WKWebView rather than the window content view. Tauri
            // resizes content-view children to the full window during layout,
            // while WebKit leaves this explicitly framed overlay unchanged.
            #[cfg(target_os = "macos")]
            let result = session.attach_to_ns_view(webview.inner(), viewport.into());
            #[cfg(target_os = "ios")]
            let result = session.attach_to_ui_view(webview.inner(), viewport.into());
            let result = result.map_err(|error| error.to_string());
            let _ = sender.send(result);
        })
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| format!("preview attachment task failed: {error}"))?
        .map_err(|_| "preview attachment channel closed".to_string())?
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn stop_preview_runtime(
    window: &tauri::WebviewWindow,
    runtime: PreviewRuntime,
) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    window
        .run_on_main_thread(move || {
            let result = runtime.host.detach().map_err(|error| error.to_string());
            let _ = sender.send((runtime.session, result));
        })
        .map_err(|error| error.to_string())?;
    let (session, detach_result) = tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| format!("preview detach task failed: {error}"))?
        .map_err(|_| "preview detach channel closed".to_string())?;
    detach_result?;
    tauri::async_runtime::spawn_blocking(move || session.stop())
        .await
        .map_err(|error| format!("camera stop task failed: {error}"))?;
    Ok(())
}

#[tauri::command]
async fn start_camera_preview(
    device_id: String,
    viewport: PreviewViewport,
    orientation: CaptureOrientation,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<PreviewStatus, String> {
    #[cfg(target_os = "android")]
    {
        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Starting)
            .map_err(|error| error.to_string())?;
        let result = android_camera::start_preview(
            window.app_handle(),
            &device_id,
            android_camera::PreviewViewport {
                x: viewport.x,
                y: viewport.y,
                width: viewport.width,
                height: viewport.height,
            },
        )
        .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let _ = state
                    .camera
                    .lock()
                    .map_err(|_| "camera state lock poisoned")?
                    .transition(CameraState::Failed);
                return Err(error);
            }
        };
        let applied_orientation =
            android_camera::set_orientation(window.app_handle(), orientation)?;
        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Previewing)
            .map_err(|error| error.to_string())?;
        return Ok(PreviewStatus {
            running: result.running,
            device_id: result.device_id,
            active_format: result.active_format.map(Into::into),
            format_restored: result.format_restored,
            settings_warning: result.settings_warning,
            orientation: applied_orientation,
        });
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .state()
            == CameraState::Recording
        {
            return Err("cannot replace camera preview while recording".into());
        }
        let previous = state
            .preview
            .lock()
            .map_err(|_| "preview state lock poisoned")?
            .take();
        if let Some(previous) = previous {
            stop_preview_runtime(&window, previous).await?;
            let mut camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() == CameraState::Previewing {
                camera
                    .transition(CameraState::Stopping)
                    .and_then(|_| camera.transition(CameraState::Idle))
                    .map_err(|error| error.to_string())?;
            }
        }

        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Starting)
            .map_err(|error| error.to_string())?;

        let requested_id = device_id.clone();
        let session = tauri::async_runtime::spawn_blocking(move || {
            AppleCaptureSession::new(&requested_id).map(Arc::new)
        })
        .await
        .map_err(|error| format!("camera configuration task failed: {error}"))?
        .map_err(|error| error.to_string())?;

        let host = match attach_preview_host(&window, Arc::clone(&session), viewport).await {
            Ok(host) => host,
            Err(error) => {
                let _ = state
                    .camera
                    .lock()
                    .map_err(|_| "camera state lock poisoned")?
                    .transition(CameraState::Failed);
                return Err(error);
            }
        };
        let start_session = Arc::clone(&session);
        tauri::async_runtime::spawn_blocking(move || start_session.start())
            .await
            .map_err(|error| format!("camera start task failed: {error}"))?;
        #[cfg(target_os = "macos")]
        let (stored_format, mut settings_warning) = match settings_path(window.app_handle())
            .and_then(|path| load_format(&path, &device_id))
        {
            Ok(format) => (format, None),
            Err(error) => (None, Some(error)),
        };
        #[cfg(target_os = "macos")]
        let mut format_restored = false;
        #[cfg(target_os = "macos")]
        let active_format = if let Some(stored) = stored_format {
            let restore_session = Arc::clone(&session);
            match tauri::async_runtime::spawn_blocking(move || {
                restore_session.set_active_format(stored.width, stored.height, stored.fps)
            })
            .await
            .map_err(|error| format!("camera format restore task failed: {error}"))?
            {
                Ok(active) => {
                    format_restored = true;
                    active
                }
                Err(error) => {
                    settings_warning =
                        Some(format!("stored camera format was not restored: {error}"));
                    session.active_format()
                }
            }
        } else {
            session.active_format()
        };
        #[cfg(target_os = "ios")]
        let (active_format, format_restored, settings_warning) =
            (session.active_format(), false, None);
        let orientation_session = Arc::clone(&session);
        let orientation = tauri::async_runtime::spawn_blocking(move || {
            orientation_session.set_capture_orientation(orientation)
        })
        .await
        .map_err(|error| format!("camera orientation task failed: {error}"))?
        .map_err(|error| error.to_string())?;

        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Previewing)
            .map_err(|error| error.to_string())?;
        *state
            .preview
            .lock()
            .map_err(|_| "preview state lock poisoned")? = Some(PreviewRuntime {
            session,
            host,
            device_id: device_id.clone(),
        });
        return Ok(PreviewStatus {
            running: true,
            device_id,
            active_format: Some(active_format.into()),
            format_restored,
            settings_warning,
            orientation,
        });
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (device_id, viewport, orientation, window, state);
        Err("native preview is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn set_camera_orientation(
    orientation: CaptureOrientation,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CaptureOrientation, String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        return android_camera::set_orientation(&app, orientation);
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let session = {
            let preview = state
                .preview
                .lock()
                .map_err(|_| "preview state lock poisoned")?;
            let Some(runtime) = preview.as_ref() else {
                return Err("camera preview is not running".into());
            };
            Arc::clone(&runtime.session)
        };
        return tauri::async_runtime::spawn_blocking(move || {
            session.set_capture_orientation(orientation)
        })
        .await
        .map_err(|error| format!("camera orientation task failed: {error}"))?
        .map_err(|error| error.to_string());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (orientation, state);
        Err("camera orientation is not implemented on this platform yet".into())
    }
}

#[tauri::command]
fn resize_camera_preview(
    viewport: PreviewViewport,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        return android_camera::resize_preview(
            window.app_handle(),
            android_camera::PreviewViewport {
                x: viewport.x,
                y: viewport.y,
                width: viewport.width,
                height: viewport.height,
            },
        );
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let preview = state
            .preview
            .lock()
            .map_err(|_| "preview state lock poisoned")?;
        let Some(runtime) = preview.as_ref() else {
            return Ok(());
        };
        let session = Arc::clone(&runtime.session);
        let host = runtime.host;
        window
            .run_on_main_thread(move || {
                #[cfg(target_os = "macos")]
                let result = session.resize_ns_view(host, viewport.into());
                #[cfg(target_os = "ios")]
                let result = session.resize_ui_view(host, viewport.into());
                let _ = result;
            })
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (viewport, window, state);
        Ok(())
    }
}

#[tauri::command]
async fn stop_camera_preview(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android_camera::stop_preview(window.app_handle())?;
        let mut camera = state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?;
        if camera.state() == CameraState::Previewing {
            camera
                .transition(CameraState::Stopping)
                .and_then(|_| camera.transition(CameraState::Idle))
                .map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .state()
            == CameraState::Recording
        {
            return Err("stop video recording before closing the preview".into());
        }
        let runtime = state
            .preview
            .lock()
            .map_err(|_| "preview state lock poisoned")?
            .take();
        if let Some(runtime) = runtime {
            stop_preview_runtime(&window, runtime).await?;
            let mut camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() == CameraState::Previewing {
                camera
                    .transition(CameraState::Stopping)
                    .and_then(|_| camera.transition(CameraState::Idle))
                    .map_err(|error| error.to_string())?;
            }
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (window, state);
        Ok(())
    }
}

#[tauri::command]
async fn capture_photo(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CapturedAsset, String> {
    #[cfg(target_os = "android")]
    {
        {
            let camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Previewing {
                return Err("camera preview is not ready".into());
            }
            if camera.mode() != CameraMode::Still {
                return Err("photo capture requires still mode".into());
            }
        }
        let captures = captures_directory(&app)?;
        ensure_capture_capacity(&captures, PHOTO_ESTIMATED_BYTES)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let id = format!("UFC-{}-{:09}", now.as_secs(), now.subsec_nanos());
        let destination = captures.join(format!("{id}.jpg"));
        let temporary = captures.join(".incomplete").join(format!("{id}.jpg"));
        let output = android_camera::capture_photo(&app, &temporary).await?;
        if output.path != temporary {
            return Err("photo output completed at an unexpected path".into());
        }
        let probe_path = output.path;
        let device_id = output.device_id;
        let active_format = output.active_format;
        let destination_for_asset = destination.clone();
        let asset_id = id.clone();
        let asset_result = tauri::async_runtime::spawn_blocking(move || {
            let resource = probe_media_resource(&probe_path, CapturedMediaType::Photo)?;
            let capture = capture_metadata_from_resource(&device_id, &resource, active_format);
            CapturedAsset::from_probed_resource(
                asset_id,
                CapturedMediaType::Photo,
                resource,
                destination_for_asset,
                capture,
            )
        })
        .await
        .map_err(|error| format!("photo validation task failed: {error}"))?;
        let asset = match asset_result {
            Ok(asset) => asset,
            Err(error) => {
                let message = error.to_string();
                let _ = MediaIndex::new(&captures).record_failed(
                    id,
                    CapturedMediaType::Photo,
                    &temporary,
                    &message,
                );
                return Err(message);
            }
        };
        finalize_captured_asset(&temporary, &destination, &asset)?;
        return Ok(asset);
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        {
            let camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Previewing {
                return Err("camera preview is not ready".into());
            }
            if camera.mode() != CameraMode::Still {
                return Err("photo capture requires still mode".into());
            }
        }

        let (session, device_id, active_format) = {
            let preview = state
                .preview
                .lock()
                .map_err(|_| "preview state lock poisoned")?;
            let runtime = preview.as_ref().ok_or("camera preview is not running")?;
            (
                Arc::clone(&runtime.session),
                runtime.device_id.clone(),
                runtime.session.active_format(),
            )
        };
        let captures = captures_directory(&app)?;
        ensure_capture_capacity(&captures, PHOTO_ESTIMATED_BYTES)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let id = format!("UFC-{}-{:09}", now.as_secs(), now.subsec_nanos());
        let destination = captures.join(format!("{id}.jpg"));
        let temporary = captures.join(".incomplete").join(format!("{id}.jpg"));
        let temporary_for_capture = temporary.clone();
        let path = tauri::async_runtime::spawn_blocking(move || {
            session.capture_photo(temporary_for_capture)
        })
        .await
        .map_err(|error| format!("photo capture task failed: {error}"))?
        .map_err(|error| error.to_string())?;
        let capture = CaptureMetadata {
            device_id,
            selected_format: selected_capture_format(active_format),
        };
        let destination_for_asset = destination.clone();
        let failure_id = id.clone();
        let asset_result = tauri::async_runtime::spawn_blocking(move || {
            let resource = probe_media_resource(&path, CapturedMediaType::Photo)?;
            CapturedAsset::from_probed_resource(
                id,
                CapturedMediaType::Photo,
                resource,
                destination_for_asset,
                capture,
            )
        })
        .await
        .map_err(|error| format!("photo validation task failed: {error}"))?;
        let asset = match asset_result {
            Ok(asset) => asset,
            Err(error) => {
                let message = error.to_string();
                let _ = MediaIndex::new(&captures).record_failed(
                    failure_id,
                    CapturedMediaType::Photo,
                    &temporary,
                    &message,
                );
                return Err(message);
            }
        };
        finalize_captured_asset(&temporary, &destination, &asset)?;
        return Ok(asset);
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (app, state);
        Err("photo capture is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn start_video_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        {
            let camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Previewing {
                return Err("camera preview is not ready".into());
            }
            if camera.mode() != CameraMode::Video {
                return Err("video recording requires video mode".into());
            }
        }
        if android_camera::microphone_authorization(&app)? != CameraAuthorizationStatus::Authorized
        {
            return Err("microphone access is not authorized".into());
        }
        let captures = captures_directory(&app)?;
        ensure_capture_capacity(&captures, VIDEO_MINIMUM_ESTIMATED_BYTES)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let id = format!("UFC-{}-{:09}", now.as_secs(), now.subsec_nanos());
        let filename = format!("{id}.mp4");
        let temporary = captures.join(".incomplete").join(&filename);
        let destination = captures.join(filename);
        android_camera::start_video(
            &app,
            &temporary,
            true,
            CAPTURE_STORAGE_RESERVE_BYTES.saturating_add(VIDEO_MINIMUM_ESTIMATED_BYTES),
        )
        .await?;
        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Recording)
            .map_err(|error| error.to_string())?;
        *state
            .pending_movie
            .lock()
            .map_err(|_| "movie state lock poisoned")? = Some(PendingMovie {
            id,
            temporary,
            destination,
        });
        return Ok(());
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        {
            let camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Previewing {
                return Err("camera preview is not ready".into());
            }
            if camera.mode() != CameraMode::Video {
                return Err("video recording requires video mode".into());
            }
        }
        if AppleCameraBackend.microphone_authorization_status()
            != CameraAuthorizationStatus::Authorized
        {
            return Err("microphone access is not authorized".into());
        }
        let (session, device_id, active_format) = {
            let preview = state
                .preview
                .lock()
                .map_err(|_| "preview state lock poisoned")?;
            let runtime = preview.as_ref().ok_or("camera preview is not running")?;
            (
                Arc::clone(&runtime.session),
                runtime.device_id.clone(),
                runtime.session.active_format(),
            )
        };
        let captures = captures_directory(&app)?;
        ensure_capture_capacity(&captures, VIDEO_MINIMUM_ESTIMATED_BYTES)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let id = format!("UFC-{}-{:09}", now.as_secs(), now.subsec_nanos());
        let monitor_id = id.clone();
        let filename = format!("{id}.mov");
        let temporary = captures.join(".incomplete").join(&filename);
        let destination = captures.join(filename);
        let recording_session = Arc::clone(&session);
        let temporary_for_task = temporary.clone();
        tauri::async_runtime::spawn_blocking(move || {
            recording_session.enable_audio_input()?;
            recording_session.start_recording(temporary_for_task)
        })
        .await
        .map_err(|error| format!("video recording start task failed: {error}"))?
        .map_err(|error| error.to_string())?;
        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Recording)
            .map_err(|error| error.to_string())?;
        *state
            .pending_movie
            .lock()
            .map_err(|_| "movie state lock poisoned")? = Some(PendingMovie {
            id,
            temporary,
            destination,
            capture: CaptureMetadata {
                device_id,
                selected_format: selected_capture_format(active_format),
            },
        });
        start_apple_storage_monitor(app, session, monitor_id, captures);
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (app, state);
        Err("video recording is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn stop_video_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CapturedAsset, String> {
    #[cfg(target_os = "android")]
    {
        {
            let mut camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Recording {
                return Err("video recording is not active".into());
            }
            camera
                .transition(CameraState::Stopping)
                .map_err(|error| error.to_string())?;
        }
        let result: Result<CapturedAsset, String> = async {
            let pending = state
                .pending_movie
                .lock()
                .map_err(|_| "movie state lock poisoned")?
                .take()
                .ok_or("movie destination is unavailable")?;
            let output = android_camera::stop_video(&app).await?;
            if output.path != pending.temporary {
                return Err("movie output completed at an unexpected path".into());
            }
            let probe_path = pending.temporary.clone();
            let destination_for_asset = pending.destination.clone();
            let asset_id = pending.id.clone();
            let device_id = output.device_id;
            let active_format = output.active_format;
            let asset_result = tauri::async_runtime::spawn_blocking(move || {
                let resource = probe_media_resource(&probe_path, CapturedMediaType::Video)?;
                let capture = capture_metadata_from_resource(&device_id, &resource, active_format);
                CapturedAsset::from_probed_resource(
                    asset_id,
                    CapturedMediaType::Video,
                    resource,
                    destination_for_asset,
                    capture,
                )
            })
            .await
            .map_err(|error| format!("movie validation task failed: {error}"))?;
            let asset = match asset_result {
                Ok(asset) => asset,
                Err(error) => {
                    let message = error.to_string();
                    let captures = pending
                        .destination
                        .parent()
                        .ok_or("movie destination has no captures directory")?;
                    let _ = MediaIndex::new(captures).record_failed(
                        pending.id,
                        CapturedMediaType::Video,
                        &pending.temporary,
                        &message,
                    );
                    return Err(message);
                }
            };
            finalize_captured_asset(&pending.temporary, &pending.destination, &asset)?;
            state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?
                .transition(CameraState::Previewing)
                .map_err(|error| error.to_string())?;
            Ok(asset)
        }
        .await;
        if result.is_err() {
            if let Ok(mut camera) = state.camera.lock() {
                let _ = camera.transition(CameraState::Failed);
            }
        }
        return result;
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let _ = app;
        {
            let mut camera = state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?;
            if camera.state() != CameraState::Recording {
                return Err("video recording is not active".into());
            }
            camera
                .transition(CameraState::Stopping)
                .map_err(|error| error.to_string())?;
        }
        let result: Result<CapturedAsset, String> = async {
            let pending = state
                .pending_movie
                .lock()
                .map_err(|_| "movie state lock poisoned")?
                .take()
                .ok_or("movie destination is unavailable")?;
            let session = {
                let preview = state
                    .preview
                    .lock()
                    .map_err(|_| "preview state lock poisoned")?;
                Arc::clone(
                    &preview
                        .as_ref()
                        .ok_or("camera preview is not running")?
                        .session,
                )
            };
            let temporary = tauri::async_runtime::spawn_blocking(move || session.stop_recording())
                .await
                .map_err(|error| format!("video recording stop task failed: {error}"))?
                .map_err(|error| error.to_string())?;
            if temporary != pending.temporary {
                return Err("movie output completed at an unexpected path".into());
            }
            let probe_path = pending.temporary.clone();
            let destination_for_asset = pending.destination.clone();
            let asset_id = pending.id.clone();
            let capture = pending.capture.clone();
            let asset_result = tauri::async_runtime::spawn_blocking(move || {
                let resource = probe_media_resource(&probe_path, CapturedMediaType::Video)?;
                CapturedAsset::from_probed_resource(
                    asset_id,
                    CapturedMediaType::Video,
                    resource,
                    destination_for_asset,
                    capture,
                )
            })
            .await
            .map_err(|error| format!("movie validation task failed: {error}"))?;
            let asset = match asset_result {
                Ok(asset) => asset,
                Err(error) => {
                    let message = error.to_string();
                    let captures = pending
                        .destination
                        .parent()
                        .ok_or("movie destination has no captures directory")?;
                    let _ = MediaIndex::new(captures).record_failed(
                        pending.id,
                        CapturedMediaType::Video,
                        &pending.temporary,
                        &message,
                    );
                    return Err(message);
                }
            };
            finalize_captured_asset(&pending.temporary, &pending.destination, &asset)?;
            state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?
                .transition(CameraState::Previewing)
                .map_err(|error| error.to_string())?;
            Ok(asset)
        }
        .await;
        if result.is_err() {
            if let Ok(mut camera) = state.camera.lock() {
                let _ = camera.transition(CameraState::Failed);
            }
        }
        return result;
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        let _ = (app, state);
        Err("video recording is not implemented on this platform yet".into())
    }
}

#[tauri::command]
fn select_camera_mode(mode: CameraMode, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .camera
        .lock()
        .map_err(|_| "camera state lock poisoned")?
        .select_mode(mode)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_imaging_pipeline_contract() -> ImagingPipelineContract {
    ImagingPipelineContract {
        schema_version: 1,
        domains: vec![
            SignalDomain::SceneLight,
            SignalDomain::OpticalImage,
            SignalDomain::FilmLatentImage,
            SignalDomain::FilmDensity,
            SignalDomain::SensorRaw,
            SignalDomain::SceneLinear,
            SignalDomain::DisplayLinear,
            SignalDomain::DisplayEncoded,
        ],
    }
}

#[tauri::command]
fn start_nearby_discovery(
    request: StartNearbyDiscoveryRequest,
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.start(request)
}

#[tauri::command]
fn get_nearby_discovery(
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.snapshot()
}

#[tauri::command]
fn stop_nearby_discovery(
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.stop()
}

#[derive(Deserialize)]
struct PrepareNearbyApprovalRequest {
    peer_ephemeral_id: String,
    asset_id: String,
}

fn now_unix_ms() -> Result<u64, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch")?
        .as_millis();
    u64::try_from(millis).map_err(|_| "system clock is outside supported range".into())
}

#[tauri::command]
fn prepare_nearby_approval(
    request: PrepareNearbyApprovalRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    let entry = MediaIndex::new(captures_directory(&app)?)
        .list()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|entry| entry.id == request.asset_id)
        .ok_or_else(|| "selected media asset was not found".to_owned())?;
    let asset = entry
        .asset
        .as_ref()
        .ok_or_else(|| "only finalized media can be shared".to_owned())?;
    state.prepare_approval(&request.peer_ephemeral_id, asset, now_unix_ms()?)
}

#[tauri::command]
fn approve_nearby_transfer(
    confirmation_code: String,
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.approve(&confirmation_code, now_unix_ms()?)
}

#[tauri::command]
fn cancel_nearby_approval(
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.cancel_approval()
}

#[tauri::command]
fn connect_nearby_transfer(
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.connect_outgoing()
}

#[tauri::command]
fn run_nearby_secure_transfer(
    app: tauri::AppHandle,
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    let captures = captures_directory(&app)?;
    let (available_bytes, _) = filesystem_capacity(&captures)?;
    state.run_secure_transfer(&captures, available_bytes)
}

#[tauri::command]
fn cancel_nearby_secure_transfer(
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    state.request_transfer_cancel()
}

#[tauri::command]
fn discard_nearby_partial(
    app: tauri::AppHandle,
    state: tauri::State<'_, NearbyDiscoveryState>,
) -> Result<NearbyDiscoverySnapshot, String> {
    let captures = captures_directory(&app)?;
    state.discard_failed_partial(&captures)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(android_camera::init());
    builder
        .manage(AppState::default())
        .manage(NearbyDiscoveryState::default())
        .invoke_handler(tauri::generate_handler![
            get_camera_status,
            get_camera_discovery,
            get_camera_capabilities,
            apply_camera_format,
            request_camera_authorization,
            get_microphone_authorization,
            request_microphone_authorization,
            start_camera_preview,
            resize_camera_preview,
            set_camera_orientation,
            stop_camera_preview,
            capture_photo,
            start_video_recording,
            stop_video_recording,
            select_camera_mode,
            get_imaging_pipeline_contract,
            get_capture_output_presets,
            get_capture_storage_status,
            get_media_index,
            reconcile_media_index,
            cleanup_media_entry,
            reinspect_media_entry,
            start_nearby_discovery,
            get_nearby_discovery,
            stop_nearby_discovery,
            prepare_nearby_approval,
            approve_nearby_transfer,
            cancel_nearby_approval,
            connect_nearby_transfer,
            run_nearby_secure_transfer,
            cancel_nearby_secure_transfer,
            discard_nearby_partial
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Universal Film Camera");
}

#[cfg(all(test, unix))]
mod storage_tests {
    use super::*;

    #[test]
    fn capture_capacity_preserves_reserve_at_the_exact_boundary() {
        let required = CAPTURE_STORAGE_RESERVE_BYTES + PHOTO_ESTIMATED_BYTES;
        assert!(!capture_capacity_ready(required - 1, PHOTO_ESTIMATED_BYTES));
        assert!(capture_capacity_ready(required, PHOTO_ESTIMATED_BYTES));
    }

    #[test]
    fn filesystem_capacity_reports_available_space_for_capture_directory() {
        let path = std::env::temp_dir().join(format!(
            "ufc-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        let (available, total) = filesystem_capacity(&path).unwrap();
        assert!(available > 0);
        assert!(total >= available);
        std::fs::remove_dir(path).unwrap();
    }
}
