use crate::{
    CaptureMedium, DevelopmentProcess, ImagingOperation, ImagingPipeline, PrintProcess,
    ProfileCatalog, ProfileEnvelope, ProfileKind, ProfileReference,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDirectoryError {
    Io {
        path: PathBuf,
        reason: String,
    },
    InvalidProfile {
        path: PathBuf,
        profile_path: String,
        reason: String,
    },
    Migration {
        path: PathBuf,
        error: MigrationError,
    },
    Empty {
        path: PathBuf,
    },
}

impl fmt::Display for ProfileDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ProfileDirectoryError {}

pub type ProfileMigrationFn = fn(Value) -> Result<Value, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedProfileMigration {
    pub name: String,
    pub from_schema_version: u32,
    pub to_schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    InvalidSchemaVersion,
    FutureSchemaVersion { found: u32, supported: u32 },
    MissingStep { from: u32, to: u32 },
    DuplicateStep { from: u32 },
    InvalidRegistration { reason: String },
    StepFailed { name: String, reason: String },
    StepProducedWrongVersion { name: String, expected: u32 },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for MigrationError {}

#[derive(Clone)]
struct MigrationStep {
    name: String,
    migrate: ProfileMigrationFn,
}

#[derive(Clone)]
pub struct ProfileMigrationRegistry {
    target_schema_version: u32,
    steps: BTreeMap<u32, MigrationStep>,
}

impl Default for ProfileMigrationRegistry {
    fn default() -> Self {
        Self {
            target_schema_version: crate::PROFILE_SCHEMA_VERSION,
            steps: BTreeMap::new(),
        }
    }
}

impl ProfileMigrationRegistry {
    pub fn register(
        &mut self,
        from_schema_version: u32,
        name: impl Into<String>,
        migrate: ProfileMigrationFn,
    ) -> Result<(), MigrationError> {
        let name = name.into();
        if name.trim().is_empty() || from_schema_version >= self.target_schema_version {
            return Err(MigrationError::InvalidRegistration {
                reason: "migration name must be non-empty and source must precede target".into(),
            });
        }
        if self.steps.contains_key(&from_schema_version) {
            return Err(MigrationError::DuplicateStep {
                from: from_schema_version,
            });
        }
        self.steps
            .insert(from_schema_version, MigrationStep { name, migrate });
        Ok(())
    }

    pub fn migrate_value(
        &self,
        mut value: Value,
    ) -> Result<(Value, Vec<AppliedProfileMigration>), MigrationError> {
        let mut version = schema_version(&value)?;
        if version > self.target_schema_version {
            return Err(MigrationError::FutureSchemaVersion {
                found: version,
                supported: self.target_schema_version,
            });
        }
        let mut applied = Vec::new();
        while version < self.target_schema_version {
            let next = version + 1;
            let step = self
                .steps
                .get(&version)
                .ok_or(MigrationError::MissingStep {
                    from: version,
                    to: next,
                })?;
            value = (step.migrate)(value).map_err(|reason| MigrationError::StepFailed {
                name: step.name.clone(),
                reason,
            })?;
            let produced = schema_version(&value)?;
            if produced != next {
                return Err(MigrationError::StepProducedWrongVersion {
                    name: step.name.clone(),
                    expected: next,
                });
            }
            applied.push(AppliedProfileMigration {
                name: step.name.clone(),
                from_schema_version: version,
                to_schema_version: next,
            });
            version = next;
        }
        Ok((value, applied))
    }
}

pub struct ProfileDirectoryLoad {
    pub catalog: ProfileCatalog,
    pub migrations: Vec<ProfileFileMigration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileFileMigration {
    pub path: PathBuf,
    pub profile_id: String,
    pub applied: Vec<AppliedProfileMigration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSnapshotEntry {
    pub id: String,
    pub kind: ProfileKind,
    pub profile_version: String,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderProfileSnapshot {
    pub schema_version: u32,
    pub pipeline_id: String,
    pub pipeline_sha256: String,
    pub profiles: Vec<ProfileSnapshotEntry>,
    pub snapshot_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidPipeline {
        reason: String,
    },
    MissingProfile {
        path: String,
        profile_id: String,
    },
    KindMismatch {
        path: String,
        profile_id: String,
        expected: ProfileKind,
        actual: ProfileKind,
    },
    ConflictingKindRequirement {
        profile_id: String,
        first: ProfileKind,
        second: ProfileKind,
    },
    InvalidProfile {
        profile_id: String,
        path: String,
        reason: String,
    },
    Serialization {
        reason: String,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for SnapshotError {}

#[derive(Debug, Clone)]
struct RequestedProfile {
    id: String,
    expected_kind: Option<ProfileKind>,
    path: String,
}

#[derive(Serialize)]
struct SnapshotPayload<'a> {
    schema_version: u32,
    pipeline_id: &'a str,
    pipeline_sha256: &'a str,
    profiles: &'a [ProfileSnapshotEntry],
}

impl ProfileCatalog {
    pub fn load_directory(root: impl AsRef<Path>) -> Result<Self, ProfileDirectoryError> {
        Ok(Self::load_directory_with_registry(root, &ProfileMigrationRegistry::default())?.catalog)
    }

    pub fn load_directory_with_registry(
        root: impl AsRef<Path>,
        registry: &ProfileMigrationRegistry,
    ) -> Result<ProfileDirectoryLoad, ProfileDirectoryError> {
        let root = root.as_ref();
        let mut paths = Vec::new();
        collect_json_paths(root, &mut paths)?;
        paths.sort();
        if paths.is_empty() {
            return Err(ProfileDirectoryError::Empty {
                path: root.to_path_buf(),
            });
        }

        let mut catalog = Self::default();
        let mut migration_report = Vec::new();
        for path in paths {
            let json = fs::read_to_string(&path).map_err(|error| ProfileDirectoryError::Io {
                path: path.clone(),
                reason: error.to_string(),
            })?;
            let value: Value = serde_json::from_str(&json).map_err(|error| {
                ProfileDirectoryError::InvalidProfile {
                    path: path.clone(),
                    profile_path: "$".into(),
                    reason: format!(
                        "invalid JSON at line {}, column {}: {}",
                        error.line(),
                        error.column(),
                        error
                    ),
                }
            })?;
            let (value, applied) = registry.migrate_value(value).map_err(|error| {
                ProfileDirectoryError::Migration {
                    path: path.clone(),
                    error,
                }
            })?;
            let normalized = serde_json::to_string(&value).map_err(|error| {
                ProfileDirectoryError::InvalidProfile {
                    path: path.clone(),
                    profile_path: "$".into(),
                    reason: error.to_string(),
                }
            })?;
            let profile = ProfileEnvelope::from_json(&normalized).map_err(|error| {
                ProfileDirectoryError::InvalidProfile {
                    path: path.clone(),
                    profile_path: error.path,
                    reason: error.reason,
                }
            })?;
            if !applied.is_empty() {
                migration_report.push(ProfileFileMigration {
                    path: path.clone(),
                    profile_id: profile.id.clone(),
                    applied,
                });
            }
            catalog
                .insert(profile)
                .map_err(|error| ProfileDirectoryError::InvalidProfile {
                    path: path.clone(),
                    profile_path: error.path,
                    reason: error.reason,
                })?;
        }
        Ok(ProfileDirectoryLoad {
            catalog,
            migrations: migration_report,
        })
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    pub fn snapshot_for_pipeline(
        &self,
        pipeline: &ImagingPipeline,
    ) -> Result<RenderProfileSnapshot, SnapshotError> {
        pipeline
            .validate()
            .map_err(|error| SnapshotError::InvalidPipeline {
                reason: error.to_string(),
            })?;

        let mut requests = VecDeque::new();
        for (index, node) in pipeline
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.enabled)
        {
            collect_node_requests(index, &node.operation, &mut requests);
        }

        let mut requirements = BTreeMap::<String, Option<ProfileKind>>::new();
        let mut selected = BTreeMap::<String, ProfileSnapshotEntry>::new();
        while let Some(request) = requests.pop_front() {
            if let Some(existing) = requirements.get(&request.id).copied().flatten()
                && let Some(incoming) = request.expected_kind
                && existing != incoming
            {
                return Err(SnapshotError::ConflictingKindRequirement {
                    profile_id: request.id,
                    first: existing,
                    second: incoming,
                });
            }
            let merged_kind = request
                .expected_kind
                .or_else(|| requirements.get(&request.id).copied().flatten());
            requirements.insert(request.id.clone(), merged_kind);
            if let Some(entry) = selected.get(&request.id) {
                if let Some(expected) = merged_kind
                    && entry.kind != expected
                {
                    return Err(SnapshotError::KindMismatch {
                        path: request.path,
                        profile_id: request.id,
                        expected,
                        actual: entry.kind,
                    });
                }
                continue;
            }

            let profile =
                self.profiles
                    .get(&request.id)
                    .ok_or_else(|| SnapshotError::MissingProfile {
                        path: request.path.clone(),
                        profile_id: request.id.clone(),
                    })?;
            if let Some(expected) = merged_kind
                && profile.kind != expected
            {
                return Err(SnapshotError::KindMismatch {
                    path: request.path,
                    profile_id: request.id,
                    expected,
                    actual: profile.kind,
                });
            }
            profile
                .validate()
                .map_err(|error| SnapshotError::InvalidProfile {
                    profile_id: profile.id.clone(),
                    path: error.path,
                    reason: error.reason,
                })?;
            let content_sha256 = canonical_sha256(profile)?;
            selected.insert(
                profile.id.clone(),
                ProfileSnapshotEntry {
                    id: profile.id.clone(),
                    kind: profile.kind,
                    profile_version: profile.profile_version.clone(),
                    content_sha256,
                },
            );
            for (reference_index, reference) in profile.references.iter().enumerate() {
                requests.push_back(request_from_reference(profile, reference_index, reference));
            }
        }

        let pipeline_sha256 = canonical_sha256(pipeline)?;
        let profiles: Vec<_> = selected.into_values().collect();
        let payload = SnapshotPayload {
            schema_version: 1,
            pipeline_id: &pipeline.id,
            pipeline_sha256: &pipeline_sha256,
            profiles: &profiles,
        };
        let snapshot_sha256 = canonical_sha256(&payload)?;
        Ok(RenderProfileSnapshot {
            schema_version: 1,
            pipeline_id: pipeline.id.clone(),
            pipeline_sha256,
            profiles,
            snapshot_sha256,
        })
    }
}

fn schema_version(value: &Value) -> Result<u32, MigrationError> {
    value
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(MigrationError::InvalidSchemaVersion)
}

fn collect_json_paths(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), ProfileDirectoryError> {
    let metadata = fs::symlink_metadata(root).map_err(|error| ProfileDirectoryError::Io {
        path: root.to_path_buf(),
        reason: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        if is_json(root) {
            paths.push(root.to_path_buf());
        }
        return Ok(());
    }
    let entries = fs::read_dir(root).map_err(|error| ProfileDirectoryError::Io {
        path: root.to_path_buf(),
        reason: error.to_string(),
    })?;
    let mut child_paths = entries
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| ProfileDirectoryError::Io {
                    path: root.to_path_buf(),
                    reason: error.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    child_paths.sort();
    for path in child_paths {
        collect_json_paths(&path, paths)?;
    }
    Ok(())
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn collect_node_requests(
    index: usize,
    operation: &ImagingOperation,
    requests: &mut VecDeque<RequestedProfile>,
) {
    let base = format!("$.nodes[{index}].operation.config");
    let mut push = |id: &str, kind: Option<ProfileKind>, field: &str| {
        if !id.trim().is_empty() {
            requests.push_back(RequestedProfile {
                id: id.into(),
                expected_kind: kind,
                path: format!("{base}.{field}"),
            });
        }
    };
    match operation {
        ImagingOperation::Source(node) => {
            if let Some(id) = &node.profile_id {
                push(id, None, "profile_id");
            }
        }
        ImagingOperation::Camera(node) => {
            push(&node.profile_id, Some(ProfileKind::Camera), "profile_id")
        }
        ImagingOperation::Lens(node) => {
            push(&node.profile_id, Some(ProfileKind::Lens), "profile_id")
        }
        ImagingOperation::VirtualExposure(_) => {}
        ImagingOperation::CaptureMedium(node) => match &node.medium {
            CaptureMedium::Film { profile_id, .. } => {
                push(profile_id, Some(ProfileKind::Film), "medium.profile_id")
            }
            CaptureMedium::DigitalSensor { profile_id, .. } => push(
                profile_id,
                Some(ProfileKind::DigitalSensor),
                "medium.profile_id",
            ),
        },
        ImagingOperation::Development(node) => match &node.process {
            DevelopmentProcess::Chemical { process_id, .. } => push(
                process_id,
                Some(ProfileKind::Development),
                "process.process_id",
            ),
            DevelopmentProcess::DigitalRaw { profile_id, .. } => push(
                profile_id,
                Some(ProfileKind::Development),
                "process.profile_id",
            ),
        },
        ImagingOperation::Print(node) => match &node.process {
            PrintProcess::Photochemical { profile_id }
            | PrintProcess::DigitalIntermediate { profile_id } => {
                push(profile_id, Some(ProfileKind::Print), "process.profile_id")
            }
        },
        ImagingOperation::OutputTransform(node) => push(
            &node.transform_id,
            Some(ProfileKind::OutputTransform),
            "transform_id",
        ),
        ImagingOperation::Display(node) => {
            push(&node.profile_id, Some(ProfileKind::Display), "profile_id")
        }
    }
}

fn request_from_reference(
    owner: &ProfileEnvelope,
    index: usize,
    reference: &ProfileReference,
) -> RequestedProfile {
    RequestedProfile {
        id: reference.profile_id.clone(),
        expected_kind: reference.expected_kind,
        path: format!("profiles[{}].references[{index}].profile_id", owner.id),
    }
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, SnapshotError> {
    let bytes = serde_json::to_vec(value).map_err(|error| SnapshotError::Serialization {
        reason: error.to_string(),
    })?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn examples_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/profiles")
    }

    fn film_emulation_pipeline() -> ImagingPipeline {
        serde_json::from_str(include_str!(
            "../../../examples/pipelines/film-emulation-reference.json"
        ))
        .unwrap()
    }

    #[test]
    fn directory_loader_recursively_loads_all_bundled_profiles() {
        let catalog = ProfileCatalog::load_directory(examples_directory()).unwrap();
        assert_eq!(catalog.len(), 8);
        catalog.validate_references().unwrap();
        let report = ProfileCatalog::load_directory_with_registry(
            examples_directory(),
            &ProfileMigrationRegistry::default(),
        )
        .unwrap();
        assert!(report.migrations.is_empty());
    }

    #[test]
    fn render_snapshot_is_sorted_complete_and_deterministic() {
        let catalog = ProfileCatalog::load_directory(examples_directory()).unwrap();
        let pipeline = film_emulation_pipeline();
        let first = catalog.snapshot_for_pipeline(&pipeline).unwrap();
        let second = catalog.snapshot_for_pipeline(&pipeline).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.profiles.len(), 4);
        assert!(first.pipeline_sha256.len() == 64 && first.snapshot_sha256.len() == 64);
        assert!(
            first
                .profiles
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
        );
        assert!(
            first
                .profiles
                .iter()
                .all(|entry| entry.content_sha256.len() == 64)
        );
    }

    #[test]
    fn render_snapshot_changes_when_profile_content_changes_without_version_change() {
        let mut catalog = ProfileCatalog::load_directory(examples_directory()).unwrap();
        let pipeline = film_emulation_pipeline();
        let before = catalog.snapshot_for_pipeline(&pipeline).unwrap();
        catalog
            .profiles
            .get_mut("org.universal-imaging.synthetic-theatrical-print")
            .unwrap()
            .model
            .push_str(" revised");
        let after = catalog.snapshot_for_pipeline(&pipeline).unwrap();
        assert_ne!(before.snapshot_sha256, after.snapshot_sha256);
        let before_entry = before
            .profiles
            .iter()
            .find(|entry| entry.id == "org.universal-imaging.synthetic-theatrical-print")
            .unwrap();
        let after_entry = after
            .profiles
            .iter()
            .find(|entry| entry.id == "org.universal-imaging.synthetic-theatrical-print")
            .unwrap();
        assert_eq!(before_entry.profile_version, after_entry.profile_version);
        assert_ne!(before_entry.content_sha256, after_entry.content_sha256);
    }

    #[test]
    fn render_snapshot_rejects_missing_direct_profile_with_pipeline_path() {
        let mut catalog = ProfileCatalog::load_directory(examples_directory()).unwrap();
        catalog
            .profiles
            .remove("org.universal-imaging.synthetic-theatrical-print");
        let error = catalog
            .snapshot_for_pipeline(&film_emulation_pipeline())
            .unwrap_err();
        assert!(matches!(
            error,
            SnapshotError::MissingProfile { ref path, .. }
                if path == "$.nodes[4].operation.config.process.profile_id"
        ));
    }

    fn migrate_test_v0_to_v1(mut value: Value) -> Result<Value, String> {
        let object = value
            .as_object_mut()
            .ok_or_else(|| "profile root must be an object".to_string())?;
        let legacy_version = object
            .remove("legacy_profile_version")
            .ok_or_else(|| "legacy_profile_version is required".to_string())?;
        object.insert("profile_version".into(), legacy_version);
        object.insert("schema_version".into(), 1.into());
        Ok(value)
    }

    fn migrate_test_wrong_version(value: Value) -> Result<Value, String> {
        Ok(value)
    }

    #[test]
    fn registered_migration_is_explicit_ordered_and_validated_afterward() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../examples/profiles/synthetic-color-negative-500.json"
        ))
        .unwrap();
        value["schema_version"] = 0.into();
        value["legacy_profile_version"] = value["profile_version"].take();

        let mut registry = ProfileMigrationRegistry::default();
        registry
            .register(0, "test-v0-to-v1", migrate_test_v0_to_v1)
            .unwrap();
        let (migrated, applied) = registry.migrate_value(value).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].name, "test-v0-to-v1");
        assert_eq!(schema_version(&migrated), Ok(1));
        let profile =
            ProfileEnvelope::from_json(&serde_json::to_string(&migrated).unwrap()).unwrap();
        assert_eq!(profile.profile_version, "1.0.0");
    }

    #[test]
    fn migration_registry_rejects_missing_and_duplicate_steps() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../examples/profiles/synthetic-color-negative-500.json"
        ))
        .unwrap();
        value["schema_version"] = 0.into();
        assert!(matches!(
            ProfileMigrationRegistry::default().migrate_value(value),
            Err(MigrationError::MissingStep { from: 0, to: 1 })
        ));

        let mut registry = ProfileMigrationRegistry::default();
        registry
            .register(0, "test-v0-to-v1", migrate_test_v0_to_v1)
            .unwrap();
        assert!(matches!(
            registry.register(0, "duplicate", migrate_test_v0_to_v1),
            Err(MigrationError::DuplicateStep { from: 0 })
        ));

        let mut future: Value = serde_json::from_str(include_str!(
            "../../../examples/profiles/synthetic-color-negative-500.json"
        ))
        .unwrap();
        future["schema_version"] = 2.into();
        assert!(matches!(
            ProfileMigrationRegistry::default().migrate_value(future),
            Err(MigrationError::FutureSchemaVersion {
                found: 2,
                supported: 1
            })
        ));

        let mut wrong_registry = ProfileMigrationRegistry::default();
        wrong_registry
            .register(0, "wrong-version", migrate_test_wrong_version)
            .unwrap();
        let mut legacy: Value = serde_json::from_str(include_str!(
            "../../../examples/profiles/synthetic-color-negative-500.json"
        ))
        .unwrap();
        legacy["schema_version"] = 0.into();
        assert!(matches!(
            wrong_registry.migrate_value(legacy),
            Err(MigrationError::StepProducedWrongVersion { expected: 1, .. })
        ));
    }
}
