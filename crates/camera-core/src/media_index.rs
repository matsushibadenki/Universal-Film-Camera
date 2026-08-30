use crate::{
    AssetState, CameraError, CapturedAsset, CapturedMediaType,
    asset::{rfc3339_from_system_time, rfc3339_now},
    probe_media_resource,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const MEDIA_RECORD_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaIndexEntry {
    pub schema_version: u32,
    pub id: String,
    pub state: AssetState,
    pub media_type: CapturedMediaType,
    pub resource_path: PathBuf,
    pub asset: Option<CapturedAsset>,
    pub error: Option<String>,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone)]
pub struct MediaIndex {
    captures_directory: PathBuf,
}

impl MediaIndex {
    pub fn new(captures_directory: impl Into<PathBuf>) -> Self {
        Self {
            captures_directory: captures_directory.into(),
        }
    }

    pub fn persist_finalized(&self, asset: &CapturedAsset) -> Result<(), CameraError> {
        if asset.state != AssetState::Finalized {
            return Err(CameraError(
                "only finalized assets can be persisted as completed media".into(),
            ));
        }
        if !asset.original.path.is_file() {
            return Err(CameraError(format!(
                "finalized media resource does not exist: {}",
                asset.original.path.display()
            )));
        }
        self.write_record(&MediaIndexEntry {
            schema_version: MEDIA_RECORD_SCHEMA_VERSION,
            id: asset.id.clone(),
            state: AssetState::Finalized,
            media_type: asset.media_type,
            resource_path: asset.original.path.clone(),
            asset: Some(asset.clone()),
            error: None,
            updated_at_utc: rfc3339_now(),
        })
    }

    pub fn record_failed(
        &self,
        id: impl Into<String>,
        media_type: CapturedMediaType,
        resource_path: impl Into<PathBuf>,
        error: impl Into<String>,
    ) -> Result<(), CameraError> {
        let id = id.into();
        let error = error.into();
        if id.trim().is_empty() || error.trim().is_empty() {
            return Err(CameraError(
                "failed media record requires an ID and error".into(),
            ));
        }
        self.write_record(&MediaIndexEntry {
            schema_version: MEDIA_RECORD_SCHEMA_VERSION,
            id,
            state: AssetState::Failed,
            media_type,
            resource_path: resource_path.into(),
            asset: None,
            error: Some(error),
            updated_at_utc: rfc3339_now(),
        })
    }

    pub fn list(&self) -> Result<Vec<MediaIndexEntry>, CameraError> {
        let mut entries = BTreeMap::new();
        let manifests = self.manifests_directory();
        if manifests.exists() {
            let mut paths = fs::read_dir(&manifests)
                .map_err(|error| CameraError(format!("failed to read media manifests: {error}")))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
                .collect::<Vec<_>>();
            paths.sort();
            for path in paths {
                let bytes = fs::read(&path).map_err(|error| {
                    CameraError(format!(
                        "failed to read media manifest {}: {error}",
                        path.display()
                    ))
                })?;
                let record: MediaIndexEntry = serde_json::from_slice(&bytes).map_err(|error| {
                    CameraError(format!(
                        "failed to parse media manifest {}: {error}",
                        path.display()
                    ))
                })?;
                validate_record(&record)?;
                if entries.insert(record.id.clone(), record).is_some() {
                    return Err(CameraError("duplicate media manifest ID".into()));
                }
            }
        }

        let incomplete = self.incomplete_directory();
        if incomplete.exists() {
            let mut paths = fs::read_dir(&incomplete)
                .map_err(|error| CameraError(format!("failed to read incomplete media: {error}")))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && !is_partial_manifest(path))
                .collect::<Vec<_>>();
            paths.sort();
            for path in paths {
                let Some((id, media_type)) = media_identity(&path) else {
                    continue;
                };
                let updated_at_utc = fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .map(rfc3339_from_system_time)
                    .unwrap_or_else(|_| rfc3339_now());
                entries.entry(id.clone()).or_insert(MediaIndexEntry {
                    schema_version: MEDIA_RECORD_SCHEMA_VERSION,
                    id,
                    state: AssetState::Incomplete,
                    media_type,
                    resource_path: path,
                    asset: None,
                    error: None,
                    updated_at_utc,
                });
            }
        }
        Ok(entries.into_values().collect())
    }

    pub fn reconcile_orphans(&self) -> Result<Vec<MediaIndexEntry>, CameraError> {
        let indexed = self.list()?;
        let finalized_paths = indexed
            .iter()
            .filter(|entry| entry.state == AssetState::Finalized)
            .map(|entry| entry.resource_path.clone())
            .collect::<Vec<_>>();
        if self.captures_directory.exists() {
            let mut paths = fs::read_dir(&self.captures_directory)
                .map_err(|error| CameraError(format!("failed to inspect captured media: {error}")))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            paths.sort();
            for path in paths {
                let Some((id, media_type)) = media_identity(&path) else {
                    continue;
                };
                if finalized_paths
                    .iter()
                    .any(|indexed_path| indexed_path == &path)
                    || indexed.iter().any(|entry| entry.id == id)
                {
                    continue;
                }
                self.record_failed(
                    id,
                    media_type,
                    path,
                    "orphaned finalized resource: manifest is missing",
                )?;
            }
        }
        self.list()
    }

    pub fn cleanup_recoverable(&self, id: &str) -> Result<(), CameraError> {
        if !is_safe_record_id(id) {
            return Err(CameraError("invalid media record ID".into()));
        }
        let entry = self
            .list()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CameraError(format!("media record was not found: {id}")))?;
        if entry.state == AssetState::Finalized {
            return Err(CameraError(
                "finalized media cannot be removed by recoverable cleanup".into(),
            ));
        }
        if entry.resource_path.exists() {
            ensure_contained_regular_file(&self.captures_directory, &entry.resource_path)?;
            fs::remove_file(&entry.resource_path).map_err(|error| {
                CameraError(format!(
                    "failed to remove recoverable media resource: {error}"
                ))
            })?;
        }
        let manifest = self.manifests_directory().join(format!("{id}.json"));
        if manifest.exists() {
            ensure_contained_regular_file(&self.manifests_directory(), &manifest)?;
            fs::remove_file(&manifest).map_err(|error| {
                CameraError(format!("failed to remove media manifest: {error}"))
            })?;
        }
        Ok(())
    }

    pub fn reinspect_recoverable(&self, id: &str) -> Result<Vec<MediaIndexEntry>, CameraError> {
        if !is_safe_record_id(id) {
            return Err(CameraError("invalid media record ID".into()));
        }
        let entry = self
            .list()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CameraError(format!("media record was not found: {id}")))?;
        if entry.state == AssetState::Finalized {
            return Err(CameraError(
                "finalized media does not require recovery reinspection".into(),
            ));
        }
        ensure_contained_regular_file(&self.captures_directory, &entry.resource_path)?;
        let diagnostic = match probe_media_resource(&entry.resource_path, entry.media_type) {
            Ok(resource) => format!(
                "reinspection passed structural media probe ({}; {}x{}; {} bytes), but the original capture intent is unavailable; recapture is recommended",
                resource.container,
                resource.pixel_width,
                resource.pixel_height,
                resource.byte_length
            ),
            Err(error) => format!("reinspection failed structural media probe: {error}"),
        };
        self.record_failed(entry.id, entry.media_type, entry.resource_path, diagnostic)?;
        self.list()
    }

    fn write_record(&self, record: &MediaIndexEntry) -> Result<(), CameraError> {
        validate_record(record)?;
        let directory = self.manifests_directory();
        fs::create_dir_all(&directory)
            .map_err(|error| CameraError(format!("failed to create media manifests: {error}")))?;
        let destination = directory.join(format!("{}.json", record.id));
        let temporary = directory.join(format!("{}.json.partial", record.id));
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|error| CameraError(format!("failed to encode media manifest: {error}")))?;
        let mut file = fs::File::create(&temporary)
            .map_err(|error| CameraError(format!("failed to create media manifest: {error}")))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| CameraError(format!("failed to write media manifest: {error}")))?;
        fs::rename(&temporary, &destination)
            .map_err(|error| CameraError(format!("failed to finalize media manifest: {error}")))
    }

    fn manifests_directory(&self) -> PathBuf {
        self.captures_directory.join(".manifests")
    }

    fn incomplete_directory(&self) -> PathBuf {
        self.captures_directory.join(".incomplete")
    }
}

fn validate_record(record: &MediaIndexEntry) -> Result<(), CameraError> {
    if record.schema_version != MEDIA_RECORD_SCHEMA_VERSION || !is_safe_record_id(&record.id) {
        return Err(CameraError("invalid media record envelope".into()));
    }
    match record.state {
        AssetState::Finalized => {
            let asset = record
                .asset
                .as_ref()
                .ok_or_else(|| CameraError("finalized media record has no asset".into()))?;
            if asset.id != record.id
                || asset.media_type != record.media_type
                || asset.state != AssetState::Finalized
                || asset.original.path != record.resource_path
                || record.error.is_some()
            {
                return Err(CameraError("finalized media record is inconsistent".into()));
            }
        }
        AssetState::Failed => {
            if record.asset.is_some() || record.error.as_deref().unwrap_or("").trim().is_empty() {
                return Err(CameraError("failed media record is inconsistent".into()));
            }
        }
        AssetState::Incomplete => {
            if record.asset.is_some() || record.error.is_some() {
                return Err(CameraError(
                    "incomplete media record is inconsistent".into(),
                ));
            }
        }
    }
    Ok(())
}

fn is_safe_record_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn ensure_contained_regular_file(root: &Path, target: &Path) -> Result<(), CameraError> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| CameraError(format!("failed to resolve media root: {error}")))?;
    let canonical_target = fs::canonicalize(target)
        .map_err(|error| CameraError(format!("failed to resolve media resource: {error}")))?;
    if !canonical_target.starts_with(&canonical_root) || !canonical_target.is_file() {
        return Err(CameraError(
            "media cleanup target is outside the managed capture directory".into(),
        ));
    }
    Ok(())
}

fn media_identity(path: &Path) -> Option<(String, CapturedMediaType)> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let media_type = match extension.as_str() {
        "jpg" | "jpeg" | "heif" | "heic" | "dng" | "png" => CapturedMediaType::Photo,
        "mov" | "mp4" => CapturedMediaType::Video,
        _ => return None,
    };
    Some((path.file_stem()?.to_str()?.to_owned(), media_type))
}

fn is_partial_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.ends_with(".json.partial"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssetValidation, CaptureMetadata, CapturedColorMetadata, MediaResource, RationalRate,
        SelectedCaptureFormat, ValidationStatus,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_directory(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ufc-media-{label}-{}-{unique}", std::process::id()))
    }

    fn finalized_asset(root: &Path) -> CapturedAsset {
        let path = root.join("asset-photo.jpg");
        fs::create_dir_all(root).unwrap();
        fs::write(&path, b"jpeg fixture").unwrap();
        CapturedAsset {
            schema_version: crate::CAPTURED_ASSET_SCHEMA_VERSION,
            id: "asset-photo".into(),
            media_type: CapturedMediaType::Photo,
            state: AssetState::Finalized,
            original_resource_id: "asset-photo:original".into(),
            original: MediaResource {
                path,
                byte_length: 12,
                container: "jpeg".into(),
                video_codec: None,
                audio_codec: None,
                pixel_width: 2,
                pixel_height: 2,
                bit_depth: Some(8),
                orientation: 1,
                orientation_explicit: false,
                rotation_degrees: 0,
                mirrored: false,
                frame_rate: None,
                duration_ms: None,
                audio_channels: None,
                audio_sample_rate_hz: None,
                color: CapturedColorMetadata::default(),
            },
            derivatives: vec![],
            capture: CaptureMetadata {
                device_id: "fixture".into(),
                selected_format: SelectedCaptureFormat {
                    width: 2,
                    height: 2,
                    fps: RationalRate {
                        numerator: 1,
                        denominator: 1,
                    },
                },
            },
            validation: AssetValidation {
                status: ValidationStatus::Passed,
                checks: vec![],
            },
            created_at_utc: "2026-08-27T00:00:00Z".into(),
        }
    }

    #[test]
    fn finalized_failed_and_incomplete_media_are_listed_separately() {
        let root = fixture_directory("states");
        let index = MediaIndex::new(&root);
        let asset = finalized_asset(&root);
        index.persist_finalized(&asset).unwrap();
        let failed_path = root.join(".incomplete/asset-video.mov");
        fs::create_dir_all(failed_path.parent().unwrap()).unwrap();
        fs::write(&failed_path, b"broken").unwrap();
        index
            .record_failed(
                "asset-video",
                CapturedMediaType::Video,
                &failed_path,
                "invalid movie",
            )
            .unwrap();
        fs::write(root.join(".incomplete/asset-pending.jpg"), b"pending").unwrap();

        let entries = index.list().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].state, AssetState::Incomplete);
        assert_eq!(entries[1].state, AssetState::Finalized);
        assert_eq!(entries[2].state, AssetState::Failed);
        assert_eq!(entries[1].asset.as_ref(), Some(&asset));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_manifest_fails_the_index_instead_of_hiding_data() {
        let root = fixture_directory("corrupt");
        let manifests = root.join(".manifests");
        fs::create_dir_all(&manifests).unwrap();
        fs::write(manifests.join("bad.json"), b"not-json").unwrap();
        assert!(index_error(&root).contains("parse media manifest"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_id_cannot_escape_the_manifest_directory() {
        let root = fixture_directory("unsafe-id");
        let error = MediaIndex::new(&root)
            .record_failed(
                "../outside",
                CapturedMediaType::Photo,
                root.join(".incomplete/file.jpg"),
                "invalid image",
            )
            .unwrap_err();
        assert!(error.to_string().contains("invalid media record envelope"));
        assert!(!root.parent().unwrap().join("outside.json").exists());
    }

    #[test]
    fn reinspection_updates_diagnostic_without_promoting_recoverable_media() {
        let root = fixture_directory("reinspect");
        let path = root.join(".incomplete/damaged.jpg");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not a jpeg").unwrap();
        let index = MediaIndex::new(&root);

        let entries = index.reinspect_recoverable("damaged").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, AssetState::Failed);
        assert!(
            entries[0]
                .error
                .as_deref()
                .unwrap()
                .contains("reinspection failed structural media probe")
        );
        assert!(path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finalized_media_cannot_enter_reinspection_recovery() {
        let root = fixture_directory("reinspect-finalized");
        let index = MediaIndex::new(&root);
        index.persist_finalized(&finalized_asset(&root)).unwrap();
        let error = index.reinspect_recoverable("asset-photo").unwrap_err();
        assert!(error.to_string().contains("does not require recovery"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reconciliation_records_orphan_without_deleting_it() {
        let root = fixture_directory("orphan");
        fs::create_dir_all(&root).unwrap();
        let orphan = root.join("orphan-photo.jpg");
        fs::write(&orphan, b"orphan").unwrap();
        let entries = MediaIndex::new(&root).reconcile_orphans().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, AssetState::Failed);
        assert!(
            entries[0]
                .error
                .as_deref()
                .unwrap()
                .contains("manifest is missing")
        );
        assert!(orphan.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup_removes_only_recoverable_managed_media() {
        let root = fixture_directory("cleanup");
        let index = MediaIndex::new(&root);
        let finalized = finalized_asset(&root);
        index.persist_finalized(&finalized).unwrap();
        assert!(
            index
                .cleanup_recoverable(&finalized.id)
                .unwrap_err()
                .to_string()
                .contains("finalized")
        );
        assert!(finalized.original.path.exists());

        let failed = root.join(".incomplete/failed-video.mov");
        fs::create_dir_all(failed.parent().unwrap()).unwrap();
        fs::write(&failed, b"broken").unwrap();
        index
            .record_failed(
                "failed-video",
                CapturedMediaType::Video,
                &failed,
                "invalid movie",
            )
            .unwrap();
        index.cleanup_recoverable("failed-video").unwrap();
        assert!(!failed.exists());
        assert!(!root.join(".manifests/failed-video.json").exists());
        assert_eq!(index.list().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    fn index_error(root: &Path) -> String {
        MediaIndex::new(root).list().unwrap_err().to_string()
    }
}
