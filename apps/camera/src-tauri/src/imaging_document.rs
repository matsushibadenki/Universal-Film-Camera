//! Rust-owned imaging decisions. Persist before publishing a new snapshot.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagingDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub selected_lut: String,
    #[serde(default)]
    pub monitoring: Option<MonitoringSettings>,
    #[serde(default)]
    pub capture_policy: Option<CapturePolicy>,
    #[serde(default)]
    pub shutter_sound: Option<ShutterSound>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShutterSound {
    #[default]
    Standard,
    Fresh,
    Dslr,
    Silent,
    Custom,
    C137,
    F2,
    H503,
    M4,
    Small,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureSettings {
    pub selected_lut: String,
    pub policy: CapturePolicy,
    pub shutter_sound: ShutterSound,
}

impl ImagingDocument {
    pub fn freeze_capture(&self) -> Result<CaptureSettings, String> {
        Ok(CaptureSettings {
            selected_lut: self.selected_lut.clone(),
            policy: self
                .capture_policy
                .clone()
                .ok_or("capture policy not initialized")?,
            shutter_sound: self.shutter_sound.ok_or("shutter sound not initialized")?,
        })
    }
}

pub fn save_shutter_sound(
    path: &Path,
    current: &ImagingDocument,
    sound: ShutterSound,
    expected_revision: u64,
) -> Result<ImagingDocument, String> {
    let mut candidate = current.clone();
    candidate.shutter_sound = Some(sound);
    save_selection(
        path,
        &candidate,
        current.selected_lut.clone(),
        expected_revision,
    )
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePolicy {
    pub move_to_photos: bool,
}

pub fn save_capture_policy(
    path: &Path,
    current: &ImagingDocument,
    policy: CapturePolicy,
    expected_revision: u64,
) -> Result<ImagingDocument, String> {
    let mut candidate = current.clone();
    candidate.capture_policy = Some(policy);
    save_selection(
        path,
        &candidate,
        current.selected_lut.clone(),
        expected_revision,
    )
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitoringSettings {
    pub color_space: String,
    pub look_enabled: bool,
    pub guide_style: String,
    pub peaking_color: String,
}

impl Default for MonitoringSettings {
    fn default() -> Self {
        Self {
            color_space: "rec709".into(),
            look_enabled: false,
            guide_style: "thirds".into(),
            peaking_color: "cyan".into(),
        }
    }
}

impl MonitoringSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !["rec709", "display-p3", "srgb"].contains(&self.color_space.as_str())
            || !["thirds", "grid", "diagonal"].contains(&self.guide_style.as_str())
            || !["cyan", "red", "green", "yellow", "magenta", "white"]
                .contains(&self.peaking_color.as_str())
        {
            return Err("invalid monitoring setting".into());
        }
        Ok(())
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitoringPatch {
    pub color_space: Option<String>,
    pub look_enabled: Option<bool>,
    pub guide_style: Option<String>,
    pub peaking_color: Option<String>,
}

pub fn save_monitoring(
    path: &Path,
    current: &ImagingDocument,
    patch: MonitoringPatch,
    expected_revision: u64,
) -> Result<ImagingDocument, String> {
    let mut settings = current.monitoring.clone().unwrap_or_default();
    if let Some(value) = patch.color_space {
        settings.color_space = value;
    }
    if let Some(value) = patch.look_enabled {
        settings.look_enabled = value;
    }
    if let Some(value) = patch.guide_style {
        settings.guide_style = value;
    }
    if let Some(value) = patch.peaking_color {
        settings.peaking_color = value;
    }
    settings.validate()?;
    let mut candidate = current.clone();
    candidate.monitoring = Some(settings);
    save_selection(
        path,
        &candidate,
        current.selected_lut.clone(),
        expected_revision,
    )
}

impl Default for ImagingDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            selected_lut: "none".into(),
            monitoring: None,
            capture_policy: None,
            shutter_sound: None,
        }
    }
}

pub fn load(path: &Path) -> Result<Option<ImagingDocument>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let document: ImagingDocument = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if document.schema_version != 1 {
        return Err("unsupported imaging document version".into());
    }
    if let Some(settings) = &document.monitoring {
        settings.validate()?;
    }
    Ok(Some(document))
}

pub fn save_selection(
    path: &Path,
    current: &ImagingDocument,
    selected_lut: String,
    expected_revision: u64,
) -> Result<ImagingDocument, String> {
    if current.revision != expected_revision {
        return Err("imaging document changed; reload settings".into());
    }
    let next = ImagingDocument {
        revision: current
            .revision
            .checked_add(1)
            .ok_or("imaging revision overflow")?,
        selected_lut,
        ..current.clone()
    };
    let parent = path.parent().ok_or("missing imaging document directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.partial");
    let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_ID: AtomicUsize = AtomicUsize::new(0);
    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ufc-document-{}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn persists_revision_and_selection() {
        let dir = directory();
        let path = dir.join("imaging.json");
        assert_eq!(load(&path).unwrap(), None);
        let current = ImagingDocument::default();
        let next = save_selection(&path, &current, "print-warm".into(), 0).unwrap();
        assert_eq!(next.revision, 1);
        assert_eq!(load(&path).unwrap(), Some(next));
        assert_eq!(current.selected_lut, "none");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failure_does_not_publish_or_replace_existing_document() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let current = save_selection(&path, &ImagingDocument::default(), "none".into(), 0).unwrap();
        std::fs::create_dir(path.with_extension("json.partial")).unwrap();
        assert!(save_selection(&path, &current, "print-warm".into(), current.revision).is_err());
        assert_eq!(load(&path).unwrap(), Some(current));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejects_corruption_and_future_versions() {
        let dir = directory();
        let path = dir.join("imaging.json");
        for bytes in [
            "{",
            r#"{"schema_version":2,"revision":0,"selected_lut":"none"}"#,
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(load(&path).is_err());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stale_window_cannot_overwrite_committed_selection() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let next =
            save_selection(&path, &ImagingDocument::default(), "print-warm".into(), 0).unwrap();
        let frozen_capture = next.selected_lut.clone();
        assert!(save_selection(&path, &next, "none".into(), 0).is_err());
        assert_eq!(load(&path).unwrap(), Some(next.clone()));
        let newer = save_selection(&path, &next, "none".into(), 1).unwrap();
        assert_eq!(newer.revision, 2);
        assert_eq!(frozen_capture, "print-warm");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn partial_monitoring_changes_preserve_lut_and_other_fields() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let first =
            save_selection(&path, &ImagingDocument::default(), "print-warm".into(), 0).unwrap();
        let next = save_monitoring(
            &path,
            &first,
            MonitoringPatch {
                color_space: Some("display-p3".into()),
                look_enabled: Some(true),
                ..Default::default()
            },
            1,
        )
        .unwrap();
        let last = save_monitoring(
            &path,
            &next,
            MonitoringPatch {
                peaking_color: Some("red".into()),
                ..Default::default()
            },
            2,
        )
        .unwrap();
        assert_eq!(last.selected_lut, "print-warm");
        let settings = last.monitoring.as_ref().unwrap();
        assert_eq!(settings.color_space, "display-p3");
        assert!(settings.look_enabled);
        assert_eq!(settings.guide_style, "thirds");
        assert_eq!(settings.peaking_color, "red");
        assert_eq!(load(&path).unwrap(), Some(last));
        assert!(save_monitoring(&path, &next, MonitoringPatch::default(), 1).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_monitoring_never_replaces_saved_document() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let first = save_selection(&path, &ImagingDocument::default(), "none".into(), 0).unwrap();
        for patch in [
            MonitoringPatch {
                color_space: Some("unknown".into()),
                ..Default::default()
            },
            MonitoringPatch {
                guide_style: Some("unknown".into()),
                ..Default::default()
            },
            MonitoringPatch {
                peaking_color: Some("unknown".into()),
                ..Default::default()
            },
        ] {
            assert!(save_monitoring(&path, &first, patch, 1).is_err());
            assert_eq!(load(&path).unwrap(), Some(first.clone()));
        }
        assert!(serde_json::from_str::<MonitoringPatch>(r#"{"unexpected":true}"#).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn legacy_lut_document_is_readable_without_overwriting_it() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let bytes = r#"{"schema_version":1,"revision":7,"selected_lut":"print-warm"}"#;
        std::fs::write(&path, bytes).unwrap();
        let loaded = load(&path).unwrap().unwrap();
        assert_eq!(loaded.revision, 7);
        assert!(loaded.monitoring.is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn capture_policy_is_opt_in_persistent_and_frozen_for_each_capture() {
        let dir = directory();
        let path = dir.join("imaging.json");
        assert!(!CapturePolicy::default().move_to_photos);
        let initial = ImagingDocument::default();
        let enabled = save_capture_policy(
            &path,
            &initial,
            CapturePolicy {
                move_to_photos: true,
            },
            0,
        )
        .unwrap();
        let recording_policy = enabled.capture_policy.clone().unwrap();
        let disabled = save_capture_policy(&path, &enabled, CapturePolicy::default(), 1).unwrap();
        assert!(recording_policy.move_to_photos);
        assert!(!disabled.capture_policy.as_ref().unwrap().move_to_photos);
        assert_eq!(load(&path).unwrap(), Some(disabled));
        assert!(save_capture_policy(&path, &enabled, CapturePolicy::default(), 0).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn capture_policy_save_failure_preserves_previous_opt_in() {
        let dir = directory();
        let path = dir.join("imaging.json");
        let original = save_capture_policy(
            &path,
            &ImagingDocument::default(),
            CapturePolicy::default(),
            0,
        )
        .unwrap();
        std::fs::create_dir(path.with_extension("json.partial")).unwrap();
        assert!(
            save_capture_policy(
                &path,
                &original,
                CapturePolicy {
                    move_to_photos: true
                },
                1
            )
            .is_err()
        );
        assert_eq!(load(&path).unwrap(), Some(original));
        assert!(serde_json::from_str::<CapturePolicy>(r#"{"move_to_photos":"true"}"#).is_err());
        assert!(
            serde_json::from_str::<CapturePolicy>(
                r#"{"move_to_photos":true,"delete_original":true}"#
            )
            .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sound_values_are_bounded_and_round_trip() {
        for id in [
            "standard", "fresh", "dslr", "silent", "custom", "c137", "f2", "h503", "m4", "small",
        ] {
            let encoded = serde_json::Value::String(id.into());
            let sound: ShutterSound = serde_json::from_value(encoded.clone()).unwrap();
            assert_eq!(serde_json::to_value(sound).unwrap(), encoded);
        }
        assert!(serde_json::from_str::<ShutterSound>(r#""../audio.wav""#).is_err());
        assert!(serde_json::from_str::<ShutterSound>(r#""unknown""#).is_err());
    }

    #[test]
    fn capture_settings_freeze_lut_transfer_and_sound_together() {
        let dir = directory();
        let path = dir.join("imaging.json");
        assert!(ImagingDocument::default().freeze_capture().is_err());
        let current = save_capture_policy(
            &path,
            &ImagingDocument::default(),
            CapturePolicy {
                move_to_photos: true,
            },
            0,
        )
        .unwrap();
        let current = save_shutter_sound(&path, &current, ShutterSound::Dslr, 1).unwrap();
        let current = save_selection(&path, &current, "print-warm".into(), 2).unwrap();
        let frozen = current.freeze_capture().unwrap();
        let newer = save_shutter_sound(&path, &current, ShutterSound::Silent, 3).unwrap();
        assert_eq!(frozen.shutter_sound, ShutterSound::Dslr);
        assert_eq!(frozen.selected_lut, "print-warm");
        assert!(frozen.policy.move_to_photos);
        assert_eq!(
            newer.freeze_capture().unwrap().shutter_sound,
            ShutterSound::Silent
        );
        assert_eq!(load(&path).unwrap(), Some(newer));
        assert!(save_shutter_sound(&path, &current, ShutterSound::Fresh, 2).is_err());
        std::fs::create_dir(path.with_extension("json.partial")).unwrap();
        assert!(save_shutter_sound(&path, &current, ShutterSound::Fresh, 3).is_err());
        assert_eq!(
            load(&path).unwrap().unwrap().shutter_sound,
            Some(ShutterSound::Silent)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
