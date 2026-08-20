//! Apple camera discovery and authorization boundary.
//!
//! AVFoundation session objects remain native and pixel data never crosses
//! the Tauri IPC boundary.

use camera_core::{
    CameraAuthorizationStatus, CameraBackend, CameraCapabilities, CameraConfig, CameraDevice,
    CameraError, CameraFormatCapability, CameraSession,
};

#[derive(Debug, Default)]
pub struct AppleCameraBackend;

#[derive(Debug, Clone, Copy)]
pub struct PreviewRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveCameraFormat {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[allow(deprecated)]
mod platform {
    use super::*;
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, ProtocolObject};
    use objc2::{AnyThread, DefinedClass, define_class, msg_send};
    use objc2_av_foundation::{
        AVAuthorizationStatus, AVCaptureDevice, AVCaptureDeviceDiscoverySession,
        AVCaptureDeviceInput, AVCaptureDevicePosition, AVCaptureDeviceTypeBuiltInWideAngleCamera,
        AVCaptureDeviceTypeExternalUnknown, AVCaptureExposureMode, AVCaptureFileOutput,
        AVCaptureFileOutputRecordingDelegate, AVCaptureFocusMode, AVCaptureMovieFileOutput,
        AVCapturePhoto, AVCapturePhotoCaptureDelegate, AVCapturePhotoOutput,
        AVCapturePhotoSettings, AVCaptureSession, AVCaptureSessionPresetInputPriority,
        AVCaptureVideoPreviewLayer, AVLayerVideoGravityResizeAspectFill, AVMediaType,
        AVMediaTypeAudio, AVMediaTypeVideo,
    };
    use objc2_core_media::{CMTimeMakeWithSeconds, CMVideoFormatDescriptionGetDimensions};
    use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol, NSString, NSURL};
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        io::Write,
        path::{Path, PathBuf},
        sync::{Mutex, mpsc},
        time::Duration,
    };

    #[derive(Debug)]
    struct PhotoCaptureState {
        sender: Option<mpsc::Sender<Result<Vec<u8>, CameraError>>>,
        result: Option<Result<Vec<u8>, CameraError>>,
    }

    #[derive(Debug)]
    struct PhotoCaptureDelegateIvars {
        state: Mutex<PhotoCaptureState>,
    }

    define_class!(
        // SAFETY: NSObject has no additional subclassing requirements. The
        // Rust ivars contain only a synchronized one-shot result sender.
        #[unsafe(super = NSObject)]
        #[ivars = PhotoCaptureDelegateIvars]
        struct PhotoCaptureDelegate;

        // SAFETY: NSObjectProtocol has no additional safety requirements.
        unsafe impl NSObjectProtocol for PhotoCaptureDelegate {}

        // SAFETY: The selector and parameter types match
        // AVCapturePhotoCaptureDelegate exactly. AVFoundation invokes this on
        // its callback queue, so the sender is protected by a Mutex.
        unsafe impl AVCapturePhotoCaptureDelegate for PhotoCaptureDelegate {
            #[unsafe(method(captureOutput:didFinishProcessingPhoto:error:))]
            fn did_finish_processing_photo(
                &self,
                _output: &AVCapturePhotoOutput,
                photo: &AVCapturePhoto,
                error: Option<&NSError>,
            ) {
                let result = if let Some(error) = error {
                    Err(CameraError(error.localizedDescription().to_string()))
                } else {
                    unsafe { photo.fileDataRepresentation() }
                        .map(|data| data.to_vec())
                        .ok_or_else(|| CameraError("photo data representation was empty".into()))
                };
                if let Ok(mut state) = self.ivars().state.lock() {
                    state.result = Some(result);
                }
            }

            #[unsafe(method(captureOutput:didFinishCaptureForResolvedSettings:error:))]
            fn did_finish_capture(
                &self,
                _output: &AVCapturePhotoOutput,
                _resolved_settings: &objc2_av_foundation::AVCaptureResolvedPhotoSettings,
                error: Option<&NSError>,
            ) {
                if let Ok(mut state) = self.ivars().state.lock() {
                    let result = if let Some(error) = error {
                        Err(CameraError(error.localizedDescription().to_string()))
                    } else {
                        state.result.take().unwrap_or_else(|| {
                            Err(CameraError(
                                "photo capture completed without image data".into(),
                            ))
                        })
                    };
                    if let Some(sender) = state.sender.take() {
                        let _ = sender.send(result);
                    }
                }
            }
        }
    );

    impl PhotoCaptureDelegate {
        fn new(sender: mpsc::Sender<Result<Vec<u8>, CameraError>>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(PhotoCaptureDelegateIvars {
                state: Mutex::new(PhotoCaptureState {
                    sender: Some(sender),
                    result: None,
                }),
            });
            // SAFETY: NSObject's init signature is valid for this subclass.
            unsafe { msg_send![super(this), init] }
        }
    }

    #[derive(Debug)]
    enum MovieRecordingEvent {
        Started,
        Finished(Result<(), CameraError>),
    }

    #[derive(Debug)]
    struct MovieRecordingDelegateIvars {
        sender: mpsc::Sender<MovieRecordingEvent>,
    }

    define_class!(
        // SAFETY: NSObject has no additional subclassing requirements. The
        // delegate only forwards lifecycle events through a Rust channel.
        #[unsafe(super = NSObject)]
        #[ivars = MovieRecordingDelegateIvars]
        struct MovieRecordingDelegate;

        // SAFETY: NSObjectProtocol has no additional safety requirements.
        unsafe impl NSObjectProtocol for MovieRecordingDelegate {}

        // SAFETY: Both selectors and signatures match
        // AVCaptureFileOutputRecordingDelegate. Callbacks may arrive on an
        // arbitrary AVFoundation queue, and mpsc::Sender is thread-safe.
        unsafe impl AVCaptureFileOutputRecordingDelegate for MovieRecordingDelegate {
            #[unsafe(method(captureOutput:didStartRecordingToOutputFileAtURL:fromConnections:))]
            fn did_start_recording(
                &self,
                _output: &AVCaptureFileOutput,
                _file_url: &NSURL,
                _connections: &NSArray<objc2_av_foundation::AVCaptureConnection>,
            ) {
                let _ = self.ivars().sender.send(MovieRecordingEvent::Started);
            }

            #[unsafe(method(captureOutput:didFinishRecordingToOutputFileAtURL:fromConnections:error:))]
            fn did_finish_recording(
                &self,
                _output: &AVCaptureFileOutput,
                _output_file_url: &NSURL,
                _connections: &NSArray<objc2_av_foundation::AVCaptureConnection>,
                error: Option<&NSError>,
            ) {
                let result = error
                    .map(|error| CameraError(error.localizedDescription().to_string()))
                    .map_or(Ok(()), Err);
                let _ = self
                    .ivars()
                    .sender
                    .send(MovieRecordingEvent::Finished(result));
            }
        }
    );

    impl MovieRecordingDelegate {
        fn new(sender: mpsc::Sender<MovieRecordingEvent>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(MovieRecordingDelegateIvars { sender });
            // SAFETY: NSObject's init signature is valid for this subclass.
            unsafe { msg_send![super(this), init] }
        }
    }

    struct MovieRecording {
        _delegate: Retained<MovieRecordingDelegate>,
        receiver: mpsc::Receiver<MovieRecordingEvent>,
        destination: PathBuf,
    }

    pub struct AppleCaptureSession {
        session: objc2::rc::Retained<AVCaptureSession>,
        video_device: objc2::rc::Retained<AVCaptureDevice>,
        photo_output: objc2::rc::Retained<AVCapturePhotoOutput>,
        movie_output: objc2::rc::Retained<AVCaptureMovieFileOutput>,
        preview_layer: objc2::rc::Retained<AVCaptureVideoPreviewLayer>,
        operation_lock: std::sync::Mutex<()>,
        audio_input_added: std::sync::Mutex<bool>,
        recording: std::sync::Mutex<Option<MovieRecording>>,
    }

    // SAFETY: AVFoundation capture sessions are designed to be configured and
    // started from a dedicated serial queue. `operation_lock` serializes every
    // blocking session mutation exposed by this type, while preview-layer and
    // AppKit view mutations are guarded by `MainThreadMarker` in their methods.
    unsafe impl Send for AppleCaptureSession {}
    unsafe impl Sync for AppleCaptureSession {}

    impl AppleCaptureSession {
        pub fn new(device_id: &str) -> Result<Self, CameraError> {
            let unique_id = NSString::from_str(device_id);
            // SAFETY: The unique ID is copied for the duration of the class
            // method and the returned device is retained.
            let device = unsafe { AVCaptureDevice::deviceWithUniqueID(&unique_id) }
                .ok_or_else(|| CameraError(format!("camera device not found: {device_id}")))?;
            // SAFETY: AVFoundation validates device availability and returns
            // an NSError rather than leaving a partially initialized input.
            let input = unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&device) }
                .map_err(|error| CameraError(error.localizedDescription().to_string()))?;
            // SAFETY: AVCaptureSession is an AnyThread Objective-C class.
            let session = unsafe { AVCaptureSession::new() };
            let photo_output = unsafe { AVCapturePhotoOutput::new() };
            let movie_output = unsafe { AVCaptureMovieFileOutput::new() };
            unsafe {
                session.beginConfiguration();
                if !session.canAddInput(&input) {
                    session.commitConfiguration();
                    return Err(CameraError(
                        "camera input is not compatible with the session".into(),
                    ));
                }
                session.addInput(&input);
                if !session.canAddOutput(&photo_output) {
                    session.commitConfiguration();
                    return Err(CameraError(
                        "photo output is not compatible with the session".into(),
                    ));
                }
                session.addOutput(&photo_output);
                if !session.canAddOutput(&movie_output) {
                    session.commitConfiguration();
                    return Err(CameraError(
                        "movie output is not compatible with the session".into(),
                    ));
                }
                session.addOutput(&movie_output);
                session.commitConfiguration();
            }
            // SAFETY: The preview layer retains the configured session.
            let preview_layer = unsafe { AVCaptureVideoPreviewLayer::layerWithSession(&session) };
            let gravity = unsafe { AVLayerVideoGravityResizeAspectFill }
                .expect("AVLayerVideoGravityResizeAspectFill must be available");
            unsafe { preview_layer.setVideoGravity(gravity) };
            Ok(Self {
                session,
                video_device: device,
                photo_output,
                movie_output,
                preview_layer,
                operation_lock: std::sync::Mutex::new(()),
                audio_input_added: std::sync::Mutex::new(false),
                recording: std::sync::Mutex::new(None),
            })
        }

        pub fn start(&self) {
            let _guard = self
                .operation_lock
                .lock()
                .expect("capture session lock poisoned");
            // SAFETY: Configuration was committed in `new`; this blocking
            // method is called from a Tauri blocking worker.
            unsafe { self.session.startRunning() };
        }

        pub fn stop(&self) {
            let _guard = self
                .operation_lock
                .lock()
                .expect("capture session lock poisoned");
            // SAFETY: AVFoundation permits stopping an idle or running session.
            unsafe { self.session.stopRunning() };
        }

        pub fn is_running(&self) -> bool {
            unsafe { self.session.isRunning() }
        }

        pub fn active_format(&self) -> ActiveCameraFormat {
            // SAFETY: The session retains the device and has completed its
            // configuration before this query. Format objects are immutable.
            let format = unsafe { self.video_device.activeFormat() };
            let description = unsafe { format.formatDescription() };
            let dimensions = unsafe { CMVideoFormatDescriptionGetDimensions(&description) };
            let duration = unsafe { self.video_device.activeVideoMinFrameDuration() };
            let fps = if duration.value > 0 && duration.timescale > 0 {
                f64::from(duration.timescale) / duration.value as f64
            } else {
                unsafe { format.videoSupportedFrameRateRanges() }
                    .firstObject()
                    .map(|range| unsafe { range.maxFrameRate() })
                    .unwrap_or(0.0)
            };
            ActiveCameraFormat {
                width: dimensions.width.max(0) as u32,
                height: dimensions.height.max(0) as u32,
                fps,
            }
        }

        pub fn set_active_format(
            &self,
            width: u32,
            height: u32,
            requested_fps: u32,
        ) -> Result<ActiveCameraFormat, CameraError> {
            let _guard = self
                .operation_lock
                .lock()
                .map_err(|_| CameraError("capture session lock poisoned".into()))?;
            if self
                .recording
                .lock()
                .map_err(|_| CameraError("movie recording lock poisoned".into()))?
                .is_some()
            {
                return Err(CameraError("cannot change format while recording".into()));
            }

            let requested = f64::from(requested_fps);
            let mut candidate = None;
            for format in unsafe { self.video_device.formats() }.iter() {
                let description = unsafe { format.formatDescription() };
                let dimensions = unsafe { CMVideoFormatDescriptionGetDimensions(&description) };
                if dimensions.width != width as i32 || dimensions.height != height as i32 {
                    continue;
                }
                for range in unsafe { format.videoSupportedFrameRateRanges() }.iter() {
                    let min = unsafe { range.minFrameRate() };
                    let max = unsafe { range.maxFrameRate() };
                    let Some(actual) = clamped_supported_frame_rate(min, max, requested) else {
                        continue;
                    };
                    let score = (max - actual).abs();
                    if candidate
                        .as_ref()
                        .is_none_or(|(_, _, best_score)| score < *best_score)
                    {
                        candidate = Some((format.clone(), actual, score));
                    }
                }
            }
            let (format, actual_fps, _) = candidate.ok_or_else(|| {
                CameraError(format!(
                    "unsupported camera format: {width}x{height} at {requested_fps} fps"
                ))
            })?;

            let input_priority = unsafe { AVCaptureSessionPresetInputPriority };
            let uses_input_priority = unsafe { self.session.canSetSessionPreset(input_priority) };
            if uses_input_priority {
                unsafe {
                    self.session.beginConfiguration();
                    self.session.setSessionPreset(input_priority);
                }
            }
            if let Err(error) = unsafe { self.video_device.lockForConfiguration() } {
                if uses_input_priority {
                    unsafe { self.session.commitConfiguration() };
                }
                return Err(CameraError(error.localizedDescription().to_string()));
            }
            // SAFETY: `actual_fps` is positive and finite because it was
            // clamped to a device-reported frame-rate range.
            let duration = unsafe { CMTimeMakeWithSeconds(1.0 / actual_fps, 60_000) };
            unsafe {
                self.video_device.setActiveFormat(&format);
                self.video_device.setActiveVideoMinFrameDuration(duration);
                self.video_device.setActiveVideoMaxFrameDuration(duration);
                self.video_device.unlockForConfiguration();
                if uses_input_priority {
                    self.session.commitConfiguration();
                }
            }
            Ok(self.active_format())
        }

        pub fn capture_photo(&self, destination: PathBuf) -> Result<PathBuf, CameraError> {
            let _guard = self
                .operation_lock
                .lock()
                .map_err(|_| CameraError("capture session lock poisoned".into()))?;
            if !self.is_running() {
                return Err(CameraError("camera preview is not running".into()));
            }

            let (sender, receiver) = mpsc::channel();
            let delegate = PhotoCaptureDelegate::new(sender);
            let settings = unsafe { AVCapturePhotoSettings::photoSettings() };
            unsafe {
                self.photo_output.capturePhotoWithSettings_delegate(
                    &settings,
                    ProtocolObject::from_ref(&*delegate),
                );
            }
            let bytes = receiver
                .recv_timeout(Duration::from_secs(30))
                .map_err(|_| CameraError("photo capture timed out".into()))??;
            write_photo_atomically(&destination, &bytes)?;
            Ok(destination)
        }

        pub fn enable_audio_input(&self) -> Result<(), CameraError> {
            let _guard = self
                .operation_lock
                .lock()
                .map_err(|_| CameraError("capture session lock poisoned".into()))?;
            let mut added = self
                .audio_input_added
                .lock()
                .map_err(|_| CameraError("audio input lock poisoned".into()))?;
            if *added {
                return Ok(());
            }
            if microphone_authorization_status() != CameraAuthorizationStatus::Authorized {
                return Err(CameraError("microphone access is not authorized".into()));
            }
            let device = unsafe { AVCaptureDevice::defaultDeviceWithMediaType(audio_media_type()) }
                .ok_or_else(|| CameraError("default microphone was not found".into()))?;
            let input = unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&device) }
                .map_err(|error| CameraError(error.localizedDescription().to_string()))?;
            unsafe {
                self.session.beginConfiguration();
                if !self.session.canAddInput(&input) {
                    self.session.commitConfiguration();
                    return Err(CameraError(
                        "microphone input is not compatible with the session".into(),
                    ));
                }
                self.session.addInput(&input);
                self.session.commitConfiguration();
            }
            *added = true;
            Ok(())
        }

        pub fn start_recording(&self, destination: PathBuf) -> Result<(), CameraError> {
            let _guard = self
                .operation_lock
                .lock()
                .map_err(|_| CameraError("capture session lock poisoned".into()))?;
            if !self.is_running() {
                return Err(CameraError("camera preview is not running".into()));
            }
            let mut recording = self
                .recording
                .lock()
                .map_err(|_| CameraError("recording state lock poisoned".into()))?;
            if recording.is_some() || unsafe { self.movie_output.isRecording() } {
                return Err(CameraError("video recording is already active".into()));
            }
            let parent = destination
                .parent()
                .ok_or_else(|| CameraError("movie destination has no parent directory".into()))?;
            fs::create_dir_all(parent).map_err(|error| {
                CameraError(format!("failed to create movie directory: {error}"))
            })?;
            if destination.exists() {
                return Err(CameraError("movie destination already exists".into()));
            }
            let path = destination
                .to_str()
                .ok_or_else(|| CameraError("movie destination is not valid UTF-8".into()))?;
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            let (sender, receiver) = mpsc::channel();
            let delegate = MovieRecordingDelegate::new(sender);
            unsafe {
                self.movie_output
                    .startRecordingToOutputFileURL_recordingDelegate(
                        &url,
                        ProtocolObject::from_ref(&*delegate),
                    );
            }
            match receiver.recv_timeout(Duration::from_secs(10)) {
                Ok(MovieRecordingEvent::Started) => {
                    *recording = Some(MovieRecording {
                        _delegate: delegate,
                        receiver,
                        destination,
                    });
                    Ok(())
                }
                Ok(MovieRecordingEvent::Finished(result)) => result
                    .and_then(|_| Err(CameraError("recording finished before it started".into()))),
                Err(_) => Err(CameraError("video recording did not start in time".into())),
            }
        }

        pub fn stop_recording(&self) -> Result<PathBuf, CameraError> {
            let _guard = self
                .operation_lock
                .lock()
                .map_err(|_| CameraError("capture session lock poisoned".into()))?;
            let runtime = self
                .recording
                .lock()
                .map_err(|_| CameraError("recording state lock poisoned".into()))?
                .take()
                .ok_or_else(|| CameraError("video recording is not active".into()))?;
            unsafe { self.movie_output.stopRecording() };
            loop {
                match runtime.receiver.recv_timeout(Duration::from_secs(30)) {
                    Ok(MovieRecordingEvent::Started) => continue,
                    Ok(MovieRecordingEvent::Finished(result)) => {
                        result?;
                        let size = fs::metadata(&runtime.destination)
                            .map_err(|error| {
                                CameraError(format!("recorded movie is unavailable: {error}"))
                            })?
                            .len();
                        if size == 0 {
                            return Err(CameraError("recorded movie is empty".into()));
                        }
                        return Ok(runtime.destination);
                    }
                    Err(_) => return Err(CameraError("video recording stop timed out".into())),
                }
            }
        }

        #[cfg(target_os = "macos")]
        pub fn attach_to_ns_view(
            &self,
            parent_view: *mut core::ffi::c_void,
            rect: PreviewRect,
        ) -> Result<MacPreviewHost, CameraError> {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSView;
            use objc2_foundation::{NSPoint, NSRect, NSSize};

            let mtm = MainThreadMarker::new().ok_or_else(|| {
                CameraError("preview view must be attached on the main thread".into())
            })?;
            if parent_view.is_null() {
                return Err(CameraError("Tauri window returned a null NSView".into()));
            }
            // SAFETY: Tauri owns this content view for the life of the window;
            // the caller invokes us synchronously on the main thread.
            let parent = unsafe { &*(parent_view.cast::<NSView>()) };
            let rect = compensate_for_window_chrome(parent, rect);
            let y = if parent.isFlipped() {
                rect.y
            } else {
                parent.bounds().size.height - rect.y - rect.height
            };
            let frame = NSRect::new(
                NSPoint::new(rect.x, y),
                NSSize::new(rect.width, rect.height),
            );
            let host = NSView::initWithFrame(mtm.alloc(), frame);
            host.setWantsLayer(true);
            let bounds = host.bounds();
            self.preview_layer.setFrame(bounds);
            self.preview_layer.setMasksToBounds(true);
            host.layer()
                .ok_or_else(|| CameraError("preview host did not create a CALayer".into()))?
                .addSublayer(&self.preview_layer);
            parent.addSubview(&host);
            Ok(MacPreviewHost {
                view: objc2::rc::Retained::into_raw(host) as usize,
            })
        }

        #[cfg(target_os = "macos")]
        pub fn resize_ns_view(
            &self,
            host: MacPreviewHost,
            rect: PreviewRect,
        ) -> Result<(), CameraError> {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSView;
            use objc2_foundation::{NSPoint, NSRect, NSSize};

            MainThreadMarker::new().ok_or_else(|| {
                CameraError("preview view must be resized on the main thread".into())
            })?;
            let view = unsafe { &*(host.view as *mut NSView) };
            let parent = unsafe { view.superview() }
                .ok_or_else(|| CameraError("preview host is detached".into()))?;
            let rect = compensate_for_window_chrome(&parent, rect);
            let y = if parent.isFlipped() {
                rect.y
            } else {
                parent.bounds().size.height - rect.y - rect.height
            };
            view.setFrame(NSRect::new(
                NSPoint::new(rect.x, y),
                NSSize::new(rect.width, rect.height),
            ));
            self.preview_layer.setFrame(view.bounds());
            Ok(())
        }
    }

    fn write_photo_atomically(destination: &Path, bytes: &[u8]) -> Result<(), CameraError> {
        let parent = destination
            .parent()
            .ok_or_else(|| CameraError("photo destination has no parent directory".into()))?;
        fs::create_dir_all(parent)
            .map_err(|error| CameraError(format!("failed to create capture directory: {error}")))?;
        let temporary = destination.with_extension("jpg.partial");
        let mut file = fs::File::create(&temporary)
            .map_err(|error| CameraError(format!("failed to create photo file: {error}")))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| CameraError(format!("failed to write photo file: {error}")))?;
        fs::rename(&temporary, destination)
            .map_err(|error| CameraError(format!("failed to finalize photo file: {error}")))?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{SystemTime, UNIX_EPOCH};

        #[test]
        fn atomic_photo_write_finalizes_without_partial_file() {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let destination = std::env::temp_dir().join(format!(
                "universal-film-camera-{}-{unique}.jpg",
                std::process::id()
            ));
            let partial = destination.with_extension("jpg.partial");
            write_photo_atomically(&destination, b"jpeg-fixture").unwrap();
            assert_eq!(fs::read(&destination).unwrap(), b"jpeg-fixture");
            assert!(!partial.exists());
            fs::remove_file(destination).unwrap();
        }
    }

    #[cfg(target_os = "macos")]
    fn compensate_for_window_chrome(
        parent: &objc2_app_kit::NSView,
        rect: PreviewRect,
    ) -> PreviewRect {
        // WKWebView reports DOM coordinates in its layout viewport, while its
        // native AppKit host spans the decorated window. AppKit is the source
        // of truth here because the title-bar height is not reliably exposed
        // through `window.outerHeight - window.innerHeight` in Tauri/WKWebView.
        let window_chrome_height = parent
            .window()
            .map(|window| {
                let window_height = window.frame().size.height;
                let content_height = window.contentLayoutRect().size.height;
                (window_height - content_height).max(0.0)
            })
            .unwrap_or(0.0);
        let chrome_height = window_chrome_height.max(parent.safeAreaInsets().top);
        PreviewRect {
            y: rect.y + chrome_height,
            ..rect
        }
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, Copy)]
    pub struct MacPreviewHost {
        view: usize,
    }

    #[cfg(target_os = "macos")]
    impl MacPreviewHost {
        pub fn detach(self) -> Result<(), CameraError> {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSView;
            MainThreadMarker::new().ok_or_else(|| {
                CameraError("preview view must be detached on the main thread".into())
            })?;
            // SAFETY: `view` comes from `Retained::into_raw` exactly once;
            // consuming this handle balances that retain after removal.
            let view = unsafe { objc2::rc::Retained::<NSView>::from_raw(self.view as *mut NSView) }
                .ok_or_else(|| CameraError("preview host pointer was null".into()))?;
            view.removeFromSuperview();
            Ok(())
        }
    }

    fn video_media_type() -> &'static AVMediaType {
        // SAFETY: AVFoundation initializes this exported constant when the
        // framework is loaded on every supported Apple target.
        unsafe { AVMediaTypeVideo }.expect("AVMediaTypeVideo must be available")
    }

    fn audio_media_type() -> &'static AVMediaType {
        // SAFETY: AVFoundation initializes this exported constant when the
        // framework is loaded on every supported Apple target.
        unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio must be available")
    }

    fn authorization_status_for(media_type: &AVMediaType) -> CameraAuthorizationStatus {
        // SAFETY: The media type is a framework-owned static and the class
        // method has no object-lifetime preconditions.
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
        match status {
            AVAuthorizationStatus::NotDetermined => CameraAuthorizationStatus::NotDetermined,
            AVAuthorizationStatus::Restricted => CameraAuthorizationStatus::Restricted,
            AVAuthorizationStatus::Denied => CameraAuthorizationStatus::Denied,
            AVAuthorizationStatus::Authorized => CameraAuthorizationStatus::Authorized,
            _ => CameraAuthorizationStatus::Unavailable,
        }
    }

    fn authorization_status() -> CameraAuthorizationStatus {
        authorization_status_for(video_media_type())
    }

    fn microphone_authorization_status() -> CameraAuthorizationStatus {
        authorization_status_for(audio_media_type())
    }

    fn request_authorization_for(
        media_type: &'static AVMediaType,
    ) -> Result<CameraAuthorizationStatus, CameraError> {
        let current = authorization_status_for(media_type);
        if current != CameraAuthorizationStatus::NotDetermined {
            return Ok(current);
        }
        let (sender, receiver) = mpsc::channel();
        let handler = RcBlock::new(move |granted: Bool| {
            let _ = sender.send(granted.as_bool());
        });
        // SAFETY: The copied block owns the sender until AVFoundation invokes
        // it. UI work is not performed on the callback queue.
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler)
        };
        receiver
            .recv_timeout(Duration::from_secs(120))
            .map(|granted| {
                if granted {
                    CameraAuthorizationStatus::Authorized
                } else {
                    CameraAuthorizationStatus::Denied
                }
            })
            .map_err(|_| CameraError("media authorization request timed out".into()))
    }

    impl AppleCameraBackend {
        pub fn microphone_authorization_status(&self) -> CameraAuthorizationStatus {
            microphone_authorization_status()
        }

        pub fn request_microphone_authorization(
            &self,
        ) -> Result<CameraAuthorizationStatus, CameraError> {
            request_authorization_for(audio_media_type())
        }
    }

    fn discovery_session() -> objc2::rc::Retained<AVCaptureDeviceDiscoverySession> {
        #[allow(deprecated)]
        let device_types = NSArray::from_slice(&[
            unsafe { AVCaptureDeviceTypeBuiltInWideAngleCamera },
            unsafe { AVCaptureDeviceTypeExternalUnknown },
        ]);
        // SAFETY: The device type array contains AVFoundation constants and
        // the returned discovery session owns its criteria.
        unsafe {
            AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
                &device_types,
                Some(video_media_type()),
                AVCaptureDevicePosition::Unspecified,
            )
        }
    }

    fn position(value: AVCaptureDevicePosition) -> camera_core::CameraPosition {
        match value {
            AVCaptureDevicePosition::Front => camera_core::CameraPosition::Front,
            AVCaptureDevicePosition::Back => camera_core::CameraPosition::Back,
            AVCaptureDevicePosition::Unspecified => camera_core::CameraPosition::External,
            _ => camera_core::CameraPosition::Unspecified,
        }
    }

    fn device_with_id(device_id: &str) -> Result<Retained<AVCaptureDevice>, CameraError> {
        let unique_id = NSString::from_str(device_id);
        unsafe { AVCaptureDevice::deviceWithUniqueID(&unique_id) }
            .ok_or_else(|| CameraError(format!("camera device not found: {device_id}")))
    }

    fn supported_integer_frame_rates(min: f64, max: f64) -> impl Iterator<Item = u32> {
        const STANDARD_RATES: [u32; 10] = [15, 23, 24, 25, 30, 48, 50, 60, 120, 240];
        STANDARD_RATES
            .into_iter()
            .filter(move |rate| f64::from(*rate) >= min - 0.11 && f64::from(*rate) <= max + 0.11)
    }

    fn clamped_supported_frame_rate(min: f64, max: f64, requested: f64) -> Option<f64> {
        (requested >= min - 0.11 && requested <= max + 0.11).then(|| requested.clamp(min, max))
    }

    impl CameraBackend for AppleCameraBackend {
        fn authorization_status(&self) -> CameraAuthorizationStatus {
            authorization_status()
        }

        fn request_authorization(&self) -> Result<CameraAuthorizationStatus, CameraError> {
            request_authorization_for(video_media_type())
        }

        fn devices(&self) -> Result<Vec<CameraDevice>, CameraError> {
            if authorization_status() != CameraAuthorizationStatus::Authorized {
                return Ok(Vec::new());
            }
            // SAFETY: The session retains the immutable NSArray for this call;
            // each property result is returned retained by objc2 bindings.
            let devices = unsafe { discovery_session().devices() };
            Ok(devices
                .iter()
                .map(|device| CameraDevice {
                    id: unsafe { device.uniqueID() }.to_string(),
                    label: unsafe { device.localizedName() }.to_string(),
                    position: position(unsafe { device.position() }),
                })
                .collect())
        }

        fn capabilities(&self, device_id: &str) -> Result<CameraCapabilities, CameraError> {
            if authorization_status() != CameraAuthorizationStatus::Authorized {
                return Err(CameraError("camera access is not authorized".into()));
            }
            let device = device_with_id(device_id)?;
            let mut resolutions = BTreeSet::new();
            let mut frame_rates = BTreeSet::new();
            let mut format_rates = BTreeMap::<(u32, u32), BTreeSet<u32>>::new();
            let mut min_iso = f32::MAX;
            let mut max_iso = f32::MIN;
            let formats = unsafe { device.formats() };
            for format in formats.iter() {
                let description = unsafe { format.formatDescription() };
                let dimensions = unsafe { CMVideoFormatDescriptionGetDimensions(&description) };
                if dimensions.width > 0 && dimensions.height > 0 {
                    let resolution = (dimensions.width as u32, dimensions.height as u32);
                    resolutions.insert(resolution);
                    format_rates.entry(resolution).or_default();
                }
                for range in unsafe { format.videoSupportedFrameRateRanges() }.iter() {
                    let range_min = unsafe { range.minFrameRate() };
                    let range_max = unsafe { range.maxFrameRate() };
                    let normalized =
                        supported_integer_frame_rates(range_min, range_max).collect::<Vec<_>>();
                    frame_rates.extend(normalized.iter().copied());
                    if dimensions.width > 0 && dimensions.height > 0 {
                        format_rates
                            .entry((dimensions.width as u32, dimensions.height as u32))
                            .or_default()
                            .extend(normalized);
                    }
                    for endpoint in [range_min, range_max] {
                        if endpoint.is_finite() && endpoint > 0.0 {
                            let endpoint = endpoint.round() as u32;
                            frame_rates.insert(endpoint);
                            if dimensions.width > 0 && dimensions.height > 0 {
                                format_rates
                                    .entry((dimensions.width as u32, dimensions.height as u32))
                                    .or_default()
                                    .insert(endpoint);
                            }
                        }
                    }
                }
                min_iso = min_iso.min(unsafe { format.minISO() });
                max_iso = max_iso.max(unsafe { format.maxISO() });
            }
            let manual_shutter =
                unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) };
            Ok(CameraCapabilities {
                supports_still: true,
                supports_video: true,
                supports_audio: true,
                resolutions: resolutions.into_iter().collect(),
                frame_rates: frame_rates.into_iter().collect(),
                formats: format_rates
                    .into_iter()
                    .map(|((width, height), frame_rates)| CameraFormatCapability {
                        width,
                        height,
                        frame_rates: frame_rates.into_iter().collect(),
                    })
                    .collect(),
                manual_iso: (manual_shutter && min_iso.is_finite() && max_iso.is_finite())
                    .then_some((min_iso, max_iso)),
                manual_shutter,
                manual_focus: unsafe { device.isFocusModeSupported(AVCaptureFocusMode::Locked) },
                raw_photo: false,
                log_video: false,
                hdr_video: false,
            })
        }

        fn open(&self, _config: CameraConfig) -> Result<Box<dyn CameraSession>, CameraError> {
            Err(CameraError(
                "Apple capture lifecycle is currently exposed through native Tauri commands".into(),
            ))
        }
    }

    #[cfg(test)]
    mod capability_tests {
        use super::{clamped_supported_frame_rate, supported_integer_frame_rates};

        #[test]
        fn maps_fractional_ranges_to_standard_ui_rates() {
            assert_eq!(
                supported_integer_frame_rates(23.976, 59.94).collect::<Vec<_>>(),
                vec![24, 25, 30, 48, 50, 60]
            );
        }

        #[test]
        fn clamps_display_rate_to_fractional_device_limit() {
            assert_eq!(clamped_supported_frame_rate(1.0, 59.94, 60.0), Some(59.94));
            assert_eq!(clamped_supported_frame_rate(1.0, 59.94, 61.0), None);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use platform::AppleCaptureSession;

#[cfg(target_os = "macos")]
pub use platform::MacPreviewHost;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
impl CameraBackend for AppleCameraBackend {
    fn devices(&self) -> Result<Vec<CameraDevice>, CameraError> {
        Ok(Vec::new())
    }

    fn capabilities(&self, _device_id: &str) -> Result<CameraCapabilities, CameraError> {
        Err(CameraError("Apple camera backend is unavailable".into()))
    }

    fn open(&self, _config: CameraConfig) -> Result<Box<dyn CameraSession>, CameraError> {
        Err(CameraError("Apple camera backend is unavailable".into()))
    }
}
