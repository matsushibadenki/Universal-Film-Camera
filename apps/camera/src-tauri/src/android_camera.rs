use camera_core::{
    CameraAuthorizationStatus, CameraCapabilities, CameraDevice, CaptureOrientation,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{
    AppHandle, Manager, Wry,
    plugin::{Builder, PluginHandle, TauriPlugin},
};

pub struct AndroidCamera(pub PluginHandle<Wry>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRequest<'a> {
    device_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRequest<'a> {
    device_id: &'a str,
    viewport: PreviewViewport,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputRequest<'a> {
    path: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoOutputRequest<'a> {
    path: &'a str,
    audio_enabled: bool,
}

#[derive(Serialize)]
struct FormatRequest {
    width: u32,
    height: u32,
    fps: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewViewport {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveryResponse {
    pub authorization: CameraAuthorizationStatus,
    pub devices: Vec<CameraDevice>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewResponse {
    pub running: bool,
    pub device_id: String,
    pub active_format: Option<PreviewFormatResponse>,
    pub format_restored: bool,
    pub settings_warning: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewFormatResponse {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub settings_persisted: bool,
    pub settings_warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorizationResponse {
    authorization: CameraAuthorizationStatus,
}

#[derive(Debug, Deserialize)]
pub struct CapturedOutput {
    pub path: PathBuf,
    pub device_id: String,
    pub active_format: Option<PreviewFormatResponse>,
}

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("android-camera")
        .setup(|app, api| {
            let handle = api.register_android_plugin("app.universalfilm.camera", "CameraPlugin")?;
            app.manage(AndroidCamera(handle));
            Ok(())
        })
        .build()
}

fn handle(app: &AppHandle) -> Result<tauri::State<'_, AndroidCamera>, String> {
    app.try_state::<AndroidCamera>()
        .ok_or_else(|| "Android CameraX plugin is not registered".into())
}

pub fn discovery(app: &AppHandle) -> Result<DiscoveryResponse, String> {
    handle(app)?
        .0
        .run_mobile_plugin("discovery", ())
        .map_err(|error| error.to_string())
}

pub async fn request_authorization(app: &AppHandle) -> Result<DiscoveryResponse, String> {
    handle(app)?
        .0
        .run_mobile_plugin_async("requestAuthorization", ())
        .await
        .map_err(|error| error.to_string())
}

pub fn microphone_authorization(app: &AppHandle) -> Result<CameraAuthorizationStatus, String> {
    handle(app)?
        .0
        .run_mobile_plugin::<AuthorizationResponse>("getMicrophoneAuthorization", ())
        .map(|response| response.authorization)
        .map_err(|error| error.to_string())
}

pub async fn request_microphone_authorization(
    app: &AppHandle,
) -> Result<CameraAuthorizationStatus, String> {
    handle(app)?
        .0
        .run_mobile_plugin_async::<AuthorizationResponse>("requestMicrophoneAuthorization", ())
        .await
        .map(|response| response.authorization)
        .map_err(|error| error.to_string())
}

pub fn capabilities(app: &AppHandle, device_id: &str) -> Result<CameraCapabilities, String> {
    handle(app)?
        .0
        .run_mobile_plugin("capabilities", DeviceRequest { device_id })
        .map_err(|error| error.to_string())
}

pub async fn start_preview(
    app: &AppHandle,
    device_id: &str,
    viewport: PreviewViewport,
) -> Result<PreviewResponse, String> {
    handle(app)?
        .0
        .run_mobile_plugin_async(
            "startPreview",
            PreviewRequest {
                device_id,
                viewport,
            },
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn apply_format(
    app: &AppHandle,
    width: u32,
    height: u32,
    fps: u32,
) -> Result<PreviewFormatResponse, String> {
    handle(app)?
        .0
        .run_mobile_plugin_async("applyFormat", FormatRequest { width, height, fps })
        .await
        .map_err(|error| error.to_string())
}

pub fn resize_preview(app: &AppHandle, viewport: PreviewViewport) -> Result<(), String> {
    handle(app)?
        .0
        .run_mobile_plugin("resizePreview", viewport)
        .map_err(|error| error.to_string())
}

pub fn set_orientation(
    app: &AppHandle,
    orientation: CaptureOrientation,
) -> Result<CaptureOrientation, String> {
    handle(app)?
        .0
        .run_mobile_plugin("setOrientation", orientation)
        .map_err(|error| error.to_string())
}

pub fn stop_preview(app: &AppHandle) -> Result<(), String> {
    handle(app)?
        .0
        .run_mobile_plugin("stopPreview", ())
        .map_err(|error| error.to_string())
}

pub async fn capture_photo(app: &AppHandle, path: &Path) -> Result<CapturedOutput, String> {
    let path = path
        .to_str()
        .ok_or_else(|| "photo output path is not valid UTF-8".to_string())?;
    handle(app)?
        .0
        .run_mobile_plugin_async::<CapturedOutput>("capturePhoto", OutputRequest { path })
        .await
        .map_err(|error| error.to_string())
}

pub async fn start_video(app: &AppHandle, path: &Path, audio_enabled: bool) -> Result<(), String> {
    let path = path
        .to_str()
        .ok_or_else(|| "video output path is not valid UTF-8".to_string())?;
    handle(app)?
        .0
        .run_mobile_plugin_async(
            "startVideo",
            VideoOutputRequest {
                path,
                audio_enabled,
            },
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn stop_video(app: &AppHandle) -> Result<CapturedOutput, String> {
    handle(app)?
        .0
        .run_mobile_plugin_async::<CapturedOutput>("stopVideo", ())
        .await
        .map_err(|error| error.to_string())
}
