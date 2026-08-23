use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCameraFormat {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CameraSettings {
    schema_version: u32,
    #[serde(default)]
    formats_by_device: BTreeMap<String, StoredCameraFormat>,
}

pub fn load_format(path: &Path, device_id: &str) -> Result<Option<StoredCameraFormat>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read camera settings: {error}"))?;
    let settings: CameraSettings = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse camera settings: {error}"))?;
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "unsupported camera settings schema version: {}",
            settings.schema_version
        ));
    }
    Ok(settings.formats_by_device.get(device_id).copied())
}

pub fn save_format(path: &Path, device_id: &str, format: StoredCameraFormat) -> Result<(), String> {
    let mut settings = if path.exists() {
        let bytes =
            fs::read(path).map_err(|error| format!("failed to read camera settings: {error}"))?;
        serde_json::from_slice::<CameraSettings>(&bytes)
            .map_err(|error| format!("failed to parse camera settings: {error}"))?
    } else {
        CameraSettings {
            schema_version: SETTINGS_SCHEMA_VERSION,
            ..Default::default()
        }
    };
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "unsupported camera settings schema version: {}",
            settings.schema_version
        ));
    }
    settings.formats_by_device.insert(device_id.into(), format);
    let parent = path
        .parent()
        .ok_or_else(|| "camera settings path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create camera settings directory: {error}"))?;
    let temporary = path.with_extension("json.partial");
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("failed to encode camera settings: {error}"))?;
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("failed to create camera settings: {error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to write camera settings: {error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("failed to finalize camera settings: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_path(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ufc-{label}-{}-{unique}.json", std::process::id()))
    }

    #[test]
    fn settings_round_trip_is_scoped_by_device() {
        let path = fixture_path("settings");
        let selected = StoredCameraFormat {
            width: 1280,
            height: 720,
            fps: 24,
        };
        save_format(&path, "camera-a", selected).unwrap();
        assert_eq!(load_format(&path, "camera-a").unwrap(), Some(selected));
        assert_eq!(load_format(&path, "camera-b").unwrap(), None);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn corrupt_settings_are_reported_instead_of_guessed() {
        let path = fixture_path("corrupt");
        fs::write(&path, b"not-json").unwrap();
        assert!(
            load_format(&path, "camera-a")
                .unwrap_err()
                .contains("parse")
        );
        fs::remove_file(path).unwrap();
    }
}
