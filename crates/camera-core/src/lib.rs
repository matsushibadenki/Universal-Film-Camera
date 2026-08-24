//! Camera-domain API shared by Tauri commands and every native backend.

use media_core::VideoFrame;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, path::PathBuf};

mod asset;
pub use asset::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraMode {
    #[default]
    Still,
    Video,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraState {
    #[default]
    Idle,
    Starting,
    Previewing,
    Recording,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraAuthorizationStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDevice {
    pub id: String,
    pub label: String,
    pub position: CameraPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraPosition {
    Front,
    Back,
    External,
    Unspecified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraFormatCapability {
    pub width: u32,
    pub height: u32,
    pub frame_rates: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCapabilities {
    pub supports_still: bool,
    pub supports_video: bool,
    pub supports_audio: bool,
    pub resolutions: Vec<(u32, u32)>,
    pub frame_rates: Vec<u32>,
    pub formats: Vec<CameraFormatCapability>,
    pub manual_iso: Option<(f32, f32)>,
    pub manual_shutter: bool,
    pub manual_focus: bool,
    pub raw_photo: bool,
    pub log_video: bool,
    pub hdr_video: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    pub device_id: String,
    pub mode: CameraMode,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub audio_enabled: bool,
}

/// Orientation requested by the UI for every native video connection.
///
/// Preview mirroring is intentionally independent from capture mirroring:
/// front-camera monitoring may be mirrored while stored media remains
/// unmirrored and interoperable with other imaging applications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureOrientation {
    pub rotation_degrees: u16,
    pub preview_mirrored: bool,
    pub capture_mirrored: bool,
}

impl CaptureOrientation {
    pub fn new(
        rotation_degrees: u16,
        preview_mirrored: bool,
        capture_mirrored: bool,
    ) -> Result<Self, CameraError> {
        if !matches!(rotation_degrees, 0 | 90 | 180 | 270) {
            return Err(CameraError(format!(
                "unsupported capture rotation: {rotation_degrees} degrees"
            )));
        }
        Ok(Self {
            rotation_degrees,
            preview_mirrored,
            capture_mirrored,
        })
    }
}

pub type CaptureResult = CapturedAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturedMediaType {
    Photo,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraError(pub String);
impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for CameraError {}

pub trait CameraBackend: Send + Sync {
    fn authorization_status(&self) -> CameraAuthorizationStatus {
        CameraAuthorizationStatus::Unavailable
    }
    fn request_authorization(&self) -> Result<CameraAuthorizationStatus, CameraError> {
        Err(CameraError(
            "camera authorization is unavailable on this backend".into(),
        ))
    }
    fn devices(&self) -> Result<Vec<CameraDevice>, CameraError>;
    fn capabilities(&self, device_id: &str) -> Result<CameraCapabilities, CameraError>;
    fn open(&self, config: CameraConfig) -> Result<Box<dyn CameraSession>, CameraError>;
}

pub trait CameraSession: Send {
    fn start_preview(&mut self) -> Result<(), CameraError>;
    fn next_frame(&mut self) -> Result<Option<VideoFrame>, CameraError>;
    fn capture_photo(&mut self, destination: PathBuf) -> Result<CaptureResult, CameraError>;
    fn start_recording(&mut self, destination: PathBuf) -> Result<(), CameraError>;
    fn stop_recording(&mut self) -> Result<CaptureResult, CameraError>;
    fn stop(&mut self) -> Result<(), CameraError>;
}

#[derive(Debug, Default)]
pub struct CameraController {
    state: CameraState,
    mode: CameraMode,
}

impl CameraController {
    pub fn state(&self) -> CameraState {
        self.state
    }
    pub fn mode(&self) -> CameraMode {
        self.mode
    }
    pub fn select_mode(&mut self, mode: CameraMode) -> Result<(), CameraError> {
        if self.state == CameraState::Recording {
            return Err(CameraError("cannot change mode while recording".into()));
        }
        self.mode = mode;
        Ok(())
    }
    pub fn transition(&mut self, next: CameraState) -> Result<(), CameraError> {
        let valid = matches!(
            (self.state, next),
            (CameraState::Idle, CameraState::Starting)
                | (
                    CameraState::Starting,
                    CameraState::Previewing | CameraState::Failed
                )
                | (
                    CameraState::Previewing,
                    CameraState::Recording | CameraState::Stopping | CameraState::Failed
                )
                | (
                    CameraState::Recording,
                    CameraState::Stopping | CameraState::Failed
                )
                | (
                    CameraState::Stopping,
                    CameraState::Idle | CameraState::Previewing | CameraState::Failed
                )
                | (CameraState::Failed, CameraState::Idle)
        );
        if !valid {
            return Err(CameraError(format!(
                "invalid transition: {:?} -> {next:?}",
                self.state
            )));
        }
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_blocks_mode_changes() {
        let mut c = CameraController::default();
        c.transition(CameraState::Starting).unwrap();
        c.transition(CameraState::Previewing).unwrap();
        c.transition(CameraState::Recording).unwrap();
        assert!(c.select_mode(CameraMode::Video).is_err());
    }
    #[test]
    fn invalid_transition_is_rejected() {
        assert!(
            CameraController::default()
                .transition(CameraState::Recording)
                .is_err()
        );
    }

    #[test]
    fn capture_orientation_rejects_non_quarter_turns() {
        assert!(CaptureOrientation::new(90, true, false).is_ok());
        assert!(CaptureOrientation::new(45, false, false).is_err());
    }
}
