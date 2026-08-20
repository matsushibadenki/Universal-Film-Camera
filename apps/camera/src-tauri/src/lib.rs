use camera_apple::AppleCameraBackend;
#[cfg(target_os = "macos")]
use camera_apple::{ActiveCameraFormat, AppleCaptureSession, MacPreviewHost, PreviewRect};
use camera_core::{
    CameraAuthorizationStatus, CameraBackend, CameraCapabilities, CameraController, CameraDevice,
    CameraMode, CameraState, CapturedMediaType,
};
use imaging_core::SignalDomain;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
#[cfg(target_os = "macos")]
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

struct AppState {
    camera: Mutex<CameraController>,
    backend: Arc<dyn CameraBackend>,
    #[cfg(target_os = "macos")]
    preview: Mutex<Option<PreviewRuntime>>,
    #[cfg(target_os = "macos")]
    pending_movie: Mutex<Option<PendingMovie>>,
}

#[cfg(target_os = "macos")]
struct PreviewRuntime {
    session: Arc<AppleCaptureSession>,
    host: MacPreviewHost,
}

#[cfg(target_os = "macos")]
struct PendingMovie {
    temporary: std::path::PathBuf,
    destination: std::path::PathBuf,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            camera: Mutex::new(CameraController::default()),
            backend: Arc::new(AppleCameraBackend),
            #[cfg(target_os = "macos")]
            preview: Mutex::new(None),
            #[cfg(target_os = "macos")]
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

#[derive(Debug, Clone, Copy, Deserialize)]
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
}

#[derive(Serialize)]
struct PreviewFormat {
    width: u32,
    height: u32,
    fps: f64,
}

#[derive(Serialize)]
struct CaptureAsset {
    path: String,
    media_type: CapturedMediaType,
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
impl From<ActiveCameraFormat> for PreviewFormat {
    fn from(value: ActiveCameraFormat) -> Self {
        Self {
            width: value.width,
            height: value.height,
            fps: value.fps,
        }
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

fn discovery(backend: &dyn CameraBackend) -> Result<CameraDiscovery, String> {
    let authorization = backend.authorization_status();
    let devices = backend.devices().map_err(|error| error.to_string())?;
    Ok(CameraDiscovery {
        authorization,
        devices,
    })
}

#[tauri::command]
fn get_camera_discovery(state: tauri::State<'_, AppState>) -> Result<CameraDiscovery, String> {
    discovery(state.backend.as_ref())
}

#[tauri::command]
fn get_camera_capabilities(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<CameraCapabilities, String> {
    state
        .backend
        .capabilities(&device_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn apply_camera_format(
    width: u32,
    height: u32,
    fps: u32,
    state: tauri::State<'_, AppState>,
) -> Result<PreviewFormat, String> {
    #[cfg(target_os = "macos")]
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
        let active = tauri::async_runtime::spawn_blocking(move || {
            session.set_active_format(width, height, fps)
        })
        .await
        .map_err(|error| format!("camera format task failed: {error}"))?
        .map_err(|error| error.to_string())?;
        return Ok(active.into());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (width, height, fps, state);
        Err("camera format selection is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn request_camera_authorization(
    state: tauri::State<'_, AppState>,
) -> Result<CameraDiscovery, String> {
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

#[tauri::command]
fn get_microphone_authorization() -> CameraAuthorizationStatus {
    #[cfg(target_os = "macos")]
    {
        return AppleCameraBackend.microphone_authorization_status();
    }
    #[cfg(not(target_os = "macos"))]
    CameraAuthorizationStatus::Unavailable
}

#[tauri::command]
async fn request_microphone_authorization() -> Result<CameraAuthorizationStatus, String> {
    #[cfg(target_os = "macos")]
    {
        return tauri::async_runtime::spawn_blocking(|| {
            AppleCameraBackend
                .request_microphone_authorization()
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| format!("microphone authorization task failed: {error}"))?;
    }
    #[cfg(not(target_os = "macos"))]
    Err("microphone authorization is unavailable on this platform".into())
}

#[cfg(target_os = "macos")]
async fn attach_preview_host(
    window: &tauri::WebviewWindow,
    session: Arc<AppleCaptureSession>,
    viewport: PreviewViewport,
) -> Result<MacPreviewHost, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    window
        .with_webview(move |webview| {
            // Attach to WKWebView rather than the window content view. Tauri
            // resizes content-view children to the full window during layout,
            // while WebKit leaves this explicitly framed overlay unchanged.
            let result = session
                .attach_to_ns_view(webview.inner(), viewport.into())
                .map_err(|error| error.to_string());
            let _ = sender.send(result);
        })
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| format!("preview attachment task failed: {error}"))?
        .map_err(|_| "preview attachment channel closed".to_string())?
}

#[cfg(target_os = "macos")]
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
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<PreviewStatus, String> {
    #[cfg(target_os = "macos")]
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
        let active_format = session.active_format();

        state
            .camera
            .lock()
            .map_err(|_| "camera state lock poisoned")?
            .transition(CameraState::Previewing)
            .map_err(|error| error.to_string())?;
        *state
            .preview
            .lock()
            .map_err(|_| "preview state lock poisoned")? = Some(PreviewRuntime { session, host });
        return Ok(PreviewStatus {
            running: true,
            device_id,
            active_format: Some(active_format.into()),
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (device_id, viewport, window, state);
        Err("native preview is not implemented on this platform yet".into())
    }
}

#[tauri::command]
fn resize_camera_preview(
    viewport: PreviewViewport,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
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
                let _ = session.resize_ns_view(host, viewport.into());
            })
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
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
    #[cfg(target_os = "macos")]
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

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, state);
        Ok(())
    }
}

#[tauri::command]
async fn capture_photo(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CaptureAsset, String> {
    #[cfg(target_os = "macos")]
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
        let captures = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?
            .join("captures");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let destination = captures.join(format!(
            "UFC-{}-{:09}.jpg",
            now.as_secs(),
            now.subsec_nanos()
        ));
        let path = tauri::async_runtime::spawn_blocking(move || session.capture_photo(destination))
            .await
            .map_err(|error| format!("photo capture task failed: {error}"))?
            .map_err(|error| error.to_string())?;
        return Ok(CaptureAsset {
            path: path.to_string_lossy().into_owned(),
            media_type: CapturedMediaType::Photo,
        });
    }

    #[cfg(not(target_os = "macos"))]
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
    #[cfg(target_os = "macos")]
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
        let captures = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?
            .join("captures");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?;
        let filename = format!("UFC-{}-{:09}.mov", now.as_secs(), now.subsec_nanos());
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
            temporary,
            destination,
        });
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, state);
        Err("video recording is not implemented on this platform yet".into())
    }
}

#[tauri::command]
async fn stop_video_recording(state: tauri::State<'_, AppState>) -> Result<CaptureAsset, String> {
    #[cfg(target_os = "macos")]
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
        let result: Result<CaptureAsset, String> = async {
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
            std::fs::rename(&pending.temporary, &pending.destination)
                .map_err(|error| format!("failed to finalize movie file: {error}"))?;
            state
                .camera
                .lock()
                .map_err(|_| "camera state lock poisoned")?
                .transition(CameraState::Previewing)
                .map_err(|error| error.to_string())?;
            Ok(CaptureAsset {
                path: pending.destination.to_string_lossy().into_owned(),
                media_type: CapturedMediaType::Video,
            })
        }
        .await;
        if result.is_err() {
            if let Ok(mut camera) = state.camera.lock() {
                let _ = camera.transition(CameraState::Failed);
            }
        }
        return result;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
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
            stop_camera_preview,
            capture_photo,
            start_video_recording,
            stop_video_recording,
            select_camera_mode,
            get_imaging_pipeline_contract
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Universal Film Camera");
}
