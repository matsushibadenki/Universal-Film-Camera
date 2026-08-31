//! Camera-domain API shared by Tauri commands and every native backend.

use media_core::VideoFrame;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, path::PathBuf};

mod asset;
mod media_index;
pub use asset::*;
pub use media_index::*;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraLifecycleEvent {
    WindowClosing,
    EnteringBackground,
    SystemSleeping,
    DeviceDisconnected,
    ReturningForeground,
    DeviceAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraRecoveryAction {
    None,
    FinalizeRecordingThenStop,
    StopPreview,
    RestartPreview,
    WaitForDevice,
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
    #[serde(default)]
    pub lens_label: Option<String>,
    #[serde(default)]
    pub lens_aperture: Option<f32>,
    #[serde(default)]
    pub current_shutter_seconds: Option<f64>,
    #[serde(default)]
    pub current_iso: Option<f32>,
    #[serde(default)]
    pub manual_white_balance: bool,
    #[serde(default)]
    pub current_white_balance_kelvin: Option<f32>,
    pub raw_photo: bool,
    pub log_video: bool,
    pub hdr_video: bool,
    /// Complete output combinations exposed by the backend. An empty list is
    /// valid for older/out-of-tree backends and means "not yet reported".
    #[serde(default)]
    pub output_formats: Vec<CaptureOutputCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureContainer {
    Jpeg,
    Heif,
    Dng,
    QuickTime,
    Mp4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureCodec {
    Jpeg,
    Hevc,
    Raw,
    H264,
    ProRes422,
    Aac,
    Pcm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCapability {
    pub codec: CaptureCodec,
    pub sample_rates_hz: Vec<u32>,
    pub channel_counts: Vec<u16>,
    pub bitrate_range_bps: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureOutputCapability {
    pub id: String,
    pub mode: CameraMode,
    pub container: CaptureContainer,
    pub video_or_still_codec: CaptureCodec,
    pub bit_depths: Vec<u16>,
    /// `None` represents a codec/container combination whose bitrate is
    /// quality- or implementation-defined (for example RAW or JPEG).
    pub bitrate_range_bps: Option<(u64, u64)>,
    pub audio: Option<AudioCapability>,
}

impl CaptureOutputCapability {
    pub fn validate(&self) -> Result<(), CameraError> {
        if self.id.trim().is_empty() || self.bit_depths.is_empty() {
            return Err(CameraError(
                "capture output capability is incomplete".into(),
            ));
        }
        if let Some((minimum, maximum)) = self.bitrate_range_bps {
            if minimum == 0 || minimum > maximum {
                return Err(CameraError("invalid capture bitrate range".into()));
            }
        }
        if self.mode == CameraMode::Still && self.audio.is_some() {
            return Err(CameraError("still output cannot declare audio".into()));
        }
        if let Some(audio) = &self.audio {
            if audio.sample_rates_hz.is_empty()
                || audio.channel_counts.is_empty()
                || audio.channel_counts.iter().any(|count| *count == 0)
            {
                return Err(CameraError("invalid audio capability".into()));
            }
        }
        Ok(())
    }
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

    pub fn recovery_action(&self, event: CameraLifecycleEvent) -> CameraRecoveryAction {
        use CameraLifecycleEvent::*;
        use CameraRecoveryAction::*;
        match (self.state, event) {
            (CameraState::Recording, WindowClosing | EnteringBackground | SystemSleeping) => {
                FinalizeRecordingThenStop
            }
            (CameraState::Recording, DeviceDisconnected) => FinalizeRecordingThenStop,
            (CameraState::Previewing, WindowClosing | EnteringBackground | SystemSleeping) => {
                StopPreview
            }
            (CameraState::Previewing, DeviceDisconnected) => WaitForDevice,
            (CameraState::Idle | CameraState::Failed, ReturningForeground | DeviceAvailable) => {
                RestartPreview
            }
            _ => None,
        }
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

    #[test]
    fn output_capability_rejects_audio_on_still_and_invalid_bitrate() {
        let mut capability = CaptureOutputCapability {
            id: "jpeg".into(),
            mode: CameraMode::Still,
            container: CaptureContainer::Jpeg,
            video_or_still_codec: CaptureCodec::Jpeg,
            bit_depths: vec![8],
            bitrate_range_bps: None,
            audio: Some(AudioCapability {
                codec: CaptureCodec::Aac,
                sample_rates_hz: vec![48_000],
                channel_counts: vec![1],
                bitrate_range_bps: Some((64_000, 256_000)),
            }),
        };
        assert!(capability.validate().is_err());
        capability.mode = CameraMode::Video;
        capability.bitrate_range_bps = Some((10, 1));
        assert!(capability.validate().is_err());
        capability.bitrate_range_bps = Some((1, 10));
        assert!(capability.validate().is_ok());
    }

    #[test]
    fn lifecycle_recovery_prioritizes_recording_finalize() {
        let mut controller = CameraController::default();
        controller.transition(CameraState::Starting).unwrap();
        controller.transition(CameraState::Previewing).unwrap();
        controller.transition(CameraState::Recording).unwrap();
        assert_eq!(
            controller.recovery_action(CameraLifecycleEvent::SystemSleeping),
            CameraRecoveryAction::FinalizeRecordingThenStop
        );
        assert_eq!(
            controller.recovery_action(CameraLifecycleEvent::DeviceDisconnected),
            CameraRecoveryAction::FinalizeRecordingThenStop
        );
    }
}
