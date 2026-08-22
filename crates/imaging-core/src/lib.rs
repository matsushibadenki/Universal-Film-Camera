//! A platform-neutral, serializable description of a complete imaging chain.
//!
//! This crate describes how an image is formed. Rendering implementations live in
//! specialized crates such as `film-core`, GPU backends, and platform adapters.

use media_core::WorkingColorSpace;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fmt,
};

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Camera,
    Lens,
    DigitalSensor,
    Film,
    Development,
    Print,
    Display,
    OutputTransform,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementQuality {
    Official,
    Measured,
    Digitized,
    Estimated,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilmType {
    ColorNegative,
    ColorPositive,
    Slide,
    BlackAndWhite,
    Intermediate,
    Print,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveInterpolation {
    MonotonicCubic,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveExtrapolation {
    Clamp,
    Linear,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitometrySample {
    pub log_exposure: f32,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitometryData {
    pub x_unit: String,
    pub y_unit: String,
    pub interpolation: CurveInterpolation,
    pub extrapolation: CurveExtrapolation,
    pub samples: Vec<SensitometrySample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilmProfileData {
    pub film_type: FilmType,
    pub nominal_exposure_index: f32,
    pub native_color_temperature_kelvin: u32,
    pub sensitometry: SensitometryData,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositiveRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LensType {
    Spherical,
    Anamorphic,
    Probe,
    Computational,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LensProfileData {
    pub lens_type: LensType,
    pub mount: String,
    pub focal_length_mm: PositiveRange,
    pub aperture_f_number: PositiveRange,
    pub minimum_focus_distance_m: f32,
    pub image_circle_diameter_mm: f32,
    #[serde(default)]
    pub transmission_t_stop: Option<f32>,
    #[serde(default)]
    pub anamorphic_squeeze: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorCfa {
    BayerRggb,
    BayerBggr,
    BayerGrbg,
    BayerGbrg,
    XTrans,
    Monochrome,
    StackedRgb,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectralSensitivitySample {
    pub wavelength_nm: u16,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigitalSensorProfileData {
    pub active_width_pixels: u32,
    pub active_height_pixels: u32,
    pub sensor_width_mm: f32,
    pub sensor_height_mm: f32,
    pub native_bit_depth: u8,
    pub cfa: SensorCfa,
    #[serde(default)]
    pub custom_cfa_pattern: Option<String>,
    pub black_level: u32,
    pub white_level: u32,
    pub base_iso: f32,
    pub iso: PositiveRange,
    #[serde(default)]
    pub spectral_sensitivity: Vec<SpectralSensitivitySample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileProvenance {
    pub quality: MeasurementQuality,
    pub source_type: String,
    pub source_reference: String,
    #[serde(default)]
    pub measurement_method: Option<String>,
    #[serde(default)]
    pub measured_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileReference {
    pub profile_id: String,
    #[serde(default)]
    pub expected_kind: Option<ProfileKind>,
}

/// Common, lossless envelope shared by every imaging profile kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileEnvelope {
    pub schema_version: u32,
    pub profile_version: String,
    pub id: String,
    pub kind: ProfileKind,
    pub manufacturer: String,
    pub model: String,
    pub license: String,
    pub created_at: String,
    pub provenance: ProfileProvenance,
    #[serde(default)]
    pub references: Vec<ProfileReference>,
    pub data: Value,
    /// Unknown same-major fields are retained when an editor round-trips a profile.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError {
    pub path: String,
    pub reason: String,
}

impl ProfileError {
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl Error for ProfileError {}

impl ProfileEnvelope {
    pub fn from_json(json: &str) -> Result<Self, ProfileError> {
        let profile: Self = serde_json::from_str(json).map_err(|error| {
            ProfileError::new(
                "$",
                format!(
                    "invalid profile JSON at line {}, column {}: {}",
                    error.line(),
                    error.column(),
                    error
                ),
            )
        })?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != PROFILE_SCHEMA_VERSION {
            return Err(ProfileError::new(
                "$.schema_version",
                format!(
                    "unsupported profile schema version {}; expected {}",
                    self.schema_version, PROFILE_SCHEMA_VERSION
                ),
            ));
        }
        validate_non_empty("$.id", &self.id)?;
        validate_semver("$.profile_version", &self.profile_version)?;
        validate_non_empty("$.manufacturer", &self.manufacturer)?;
        validate_non_empty("$.model", &self.model)?;
        validate_non_empty("$.license", &self.license)?;
        validate_rfc3339("$.created_at", &self.created_at)?;
        validate_non_empty("$.provenance.source_type", &self.provenance.source_type)?;
        validate_non_empty(
            "$.provenance.source_reference",
            &self.provenance.source_reference,
        )?;
        if !self.data.is_object() {
            return Err(ProfileError::new("$.data", "must be a JSON object"));
        }
        for (index, reference) in self.references.iter().enumerate() {
            validate_non_empty(
                format!("$.references[{index}].profile_id"),
                &reference.profile_id,
            )?;
            if reference.profile_id == self.id {
                return Err(ProfileError::new(
                    format!("$.references[{index}].profile_id"),
                    "a profile cannot reference itself",
                ));
            }
        }
        if self.kind == ProfileKind::Film {
            self.film_data()?;
        } else if self.kind == ProfileKind::Lens {
            self.lens_data()?;
        } else if self.kind == ProfileKind::DigitalSensor {
            self.digital_sensor_data()?;
        }
        Ok(())
    }

    pub fn film_data(&self) -> Result<FilmProfileData, ProfileError> {
        if self.kind != ProfileKind::Film {
            return Err(ProfileError::new(
                "$.kind",
                format!("expected film profile, found {:?}", self.kind),
            ));
        }
        let data: FilmProfileData = serde_json::from_value(self.data.clone()).map_err(|error| {
            ProfileError::new("$.data", format!("invalid film profile data: {error}"))
        })?;
        data.validate()?;
        Ok(data)
    }

    pub fn lens_data(&self) -> Result<LensProfileData, ProfileError> {
        if self.kind != ProfileKind::Lens {
            return Err(ProfileError::new(
                "$.kind",
                format!("expected lens profile, found {:?}", self.kind),
            ));
        }
        let data: LensProfileData = serde_json::from_value(self.data.clone()).map_err(|error| {
            ProfileError::new("$.data", format!("invalid lens profile data: {error}"))
        })?;
        data.validate()?;
        Ok(data)
    }

    pub fn digital_sensor_data(&self) -> Result<DigitalSensorProfileData, ProfileError> {
        if self.kind != ProfileKind::DigitalSensor {
            return Err(ProfileError::new(
                "$.kind",
                format!("expected digital sensor profile, found {:?}", self.kind),
            ));
        }
        let data: DigitalSensorProfileData =
            serde_json::from_value(self.data.clone()).map_err(|error| {
                ProfileError::new(
                    "$.data",
                    format!("invalid digital sensor profile data: {error}"),
                )
            })?;
        data.validate()?;
        Ok(data)
    }
}

impl FilmProfileData {
    pub fn validate(&self) -> Result<(), ProfileError> {
        validate_positive_finite("$.data.nominal_exposure_index", self.nominal_exposure_index)?;
        if self.native_color_temperature_kelvin == 0 {
            return Err(ProfileError::new(
                "$.data.native_color_temperature_kelvin",
                "must be greater than zero",
            ));
        }
        if self.sensitometry.x_unit != "log10_lux_seconds" {
            return Err(ProfileError::new(
                "$.data.sensitometry.x_unit",
                "must be log10_lux_seconds for schema version 1",
            ));
        }
        if self.sensitometry.y_unit != "log10_optical_density" {
            return Err(ProfileError::new(
                "$.data.sensitometry.y_unit",
                "must be log10_optical_density for schema version 1",
            ));
        }
        if self.sensitometry.samples.len() < 2 {
            return Err(ProfileError::new(
                "$.data.sensitometry.samples",
                "must contain at least two samples",
            ));
        }
        let mut previous_exposure = None;
        for (index, sample) in self.sensitometry.samples.iter().enumerate() {
            let base = format!("$.data.sensitometry.samples[{index}]");
            validate_finite(format!("{base}.log_exposure"), sample.log_exposure)?;
            validate_finite(format!("{base}.red"), sample.red)?;
            validate_finite(format!("{base}.green"), sample.green)?;
            validate_finite(format!("{base}.blue"), sample.blue)?;
            if sample.red < 0.0 || sample.green < 0.0 || sample.blue < 0.0 {
                return Err(ProfileError::new(
                    base,
                    "optical density channels must be zero or greater",
                ));
            }
            if previous_exposure.is_some_and(|previous| sample.log_exposure <= previous) {
                return Err(ProfileError::new(
                    format!("{base}.log_exposure"),
                    "must be strictly greater than the previous sample",
                ));
            }
            previous_exposure = Some(sample.log_exposure);
        }
        Ok(())
    }
}

impl LensProfileData {
    pub fn validate(&self) -> Result<(), ProfileError> {
        validate_non_empty("$.data.mount", &self.mount)?;
        validate_positive_range("$.data.focal_length_mm", self.focal_length_mm)?;
        validate_positive_range("$.data.aperture_f_number", self.aperture_f_number)?;
        validate_positive_finite(
            "$.data.minimum_focus_distance_m",
            self.minimum_focus_distance_m,
        )?;
        validate_positive_finite(
            "$.data.image_circle_diameter_mm",
            self.image_circle_diameter_mm,
        )?;
        if let Some(t_stop) = self.transmission_t_stop {
            validate_positive_finite("$.data.transmission_t_stop", t_stop)?;
        }
        match (self.lens_type, self.anamorphic_squeeze) {
            (LensType::Anamorphic, Some(squeeze)) => {
                validate_positive_finite("$.data.anamorphic_squeeze", squeeze)?;
                if squeeze <= 1.0 {
                    return Err(ProfileError::new(
                        "$.data.anamorphic_squeeze",
                        "must be greater than 1 for an anamorphic lens",
                    ));
                }
            }
            (LensType::Anamorphic, None) => {
                return Err(ProfileError::new(
                    "$.data.anamorphic_squeeze",
                    "is required for an anamorphic lens",
                ));
            }
            (_, Some(_)) => {
                return Err(ProfileError::new(
                    "$.data.anamorphic_squeeze",
                    "is only valid for an anamorphic lens",
                ));
            }
            (_, None) => {}
        }
        Ok(())
    }
}

impl DigitalSensorProfileData {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.active_width_pixels == 0 {
            return Err(ProfileError::new(
                "$.data.active_width_pixels",
                "must be greater than zero",
            ));
        }
        if self.active_height_pixels == 0 {
            return Err(ProfileError::new(
                "$.data.active_height_pixels",
                "must be greater than zero",
            ));
        }
        validate_positive_finite("$.data.sensor_width_mm", self.sensor_width_mm)?;
        validate_positive_finite("$.data.sensor_height_mm", self.sensor_height_mm)?;
        if !(1..=32).contains(&self.native_bit_depth) {
            return Err(ProfileError::new(
                "$.data.native_bit_depth",
                "must be between 1 and 32",
            ));
        }
        match (self.cfa, self.custom_cfa_pattern.as_deref()) {
            (SensorCfa::Custom, Some(pattern)) if !pattern.trim().is_empty() => {}
            (SensorCfa::Custom, _) => {
                return Err(ProfileError::new(
                    "$.data.custom_cfa_pattern",
                    "is required when cfa is custom",
                ));
            }
            (_, Some(_)) => {
                return Err(ProfileError::new(
                    "$.data.custom_cfa_pattern",
                    "is only valid when cfa is custom",
                ));
            }
            (_, None) => {}
        }
        if self.white_level <= self.black_level {
            return Err(ProfileError::new(
                "$.data.white_level",
                "must be greater than black_level",
            ));
        }
        let maximum_code = if self.native_bit_depth == 32 {
            u32::MAX
        } else {
            (1_u32 << self.native_bit_depth) - 1
        };
        if self.white_level > maximum_code {
            return Err(ProfileError::new(
                "$.data.white_level",
                "exceeds the native bit-depth code range",
            ));
        }
        validate_positive_finite("$.data.base_iso", self.base_iso)?;
        validate_positive_range("$.data.iso", self.iso)?;
        if self.base_iso < self.iso.min || self.base_iso > self.iso.max {
            return Err(ProfileError::new(
                "$.data.base_iso",
                "must be inside the declared ISO range",
            ));
        }
        if self.spectral_sensitivity.len() == 1 {
            return Err(ProfileError::new(
                "$.data.spectral_sensitivity",
                "must be empty or contain at least two samples",
            ));
        }
        let mut previous_wavelength = None;
        for (index, sample) in self.spectral_sensitivity.iter().enumerate() {
            let base = format!("$.data.spectral_sensitivity[{index}]");
            if !(360..=830).contains(&sample.wavelength_nm) {
                return Err(ProfileError::new(
                    format!("{base}.wavelength_nm"),
                    "must be between 360 and 830 nm",
                ));
            }
            for (channel, value) in [
                ("red", sample.red),
                ("green", sample.green),
                ("blue", sample.blue),
            ] {
                validate_finite(format!("{base}.{channel}"), value)?;
                if value < 0.0 {
                    return Err(ProfileError::new(
                        format!("{base}.{channel}"),
                        "spectral sensitivity must be zero or greater",
                    ));
                }
            }
            if previous_wavelength.is_some_and(|previous| sample.wavelength_nm <= previous) {
                return Err(ProfileError::new(
                    format!("{base}.wavelength_nm"),
                    "must be strictly greater than the previous sample",
                ));
            }
            previous_wavelength = Some(sample.wavelength_nm);
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ProfileCatalog {
    profiles: BTreeMap<String, ProfileEnvelope>,
}

impl ProfileCatalog {
    pub fn insert(&mut self, profile: ProfileEnvelope) -> Result<(), ProfileError> {
        profile.validate()?;
        if self.profiles.contains_key(&profile.id) {
            return Err(ProfileError::new(
                "$.id",
                format!("duplicate profile id: {}", profile.id),
            ));
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&ProfileEnvelope> {
        self.profiles.get(id)
    }

    pub fn validate_references(&self) -> Result<(), ProfileError> {
        for profile in self.profiles.values() {
            for (index, reference) in profile.references.iter().enumerate() {
                let path = format!("profiles[{}].references[{index}]", profile.id);
                let target = self.profiles.get(&reference.profile_id).ok_or_else(|| {
                    ProfileError::new(
                        format!("{path}.profile_id"),
                        format!("referenced profile not found: {}", reference.profile_id),
                    )
                })?;
                if let Some(expected) = reference.expected_kind
                    && target.kind != expected
                {
                    return Err(ProfileError::new(
                        format!("{path}.expected_kind"),
                        format!(
                            "expected {:?}, found {:?} for {}",
                            expected, target.kind, reference.profile_id
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_non_empty(path: impl Into<String>, value: &str) -> Result<(), ProfileError> {
    if value.trim().is_empty() {
        return Err(ProfileError::new(path, "must not be empty"));
    }
    Ok(())
}

fn validate_finite(path: impl Into<String>, value: f32) -> Result<(), ProfileError> {
    if !value.is_finite() {
        return Err(ProfileError::new(path, "must be finite"));
    }
    Ok(())
}

fn validate_positive_finite(path: impl Into<String>, value: f32) -> Result<(), ProfileError> {
    let path = path.into();
    validate_finite(path.clone(), value)?;
    if value <= 0.0 {
        return Err(ProfileError::new(path, "must be greater than zero"));
    }
    Ok(())
}

fn validate_positive_range(path: &str, range: PositiveRange) -> Result<(), ProfileError> {
    validate_positive_finite(format!("{path}.min"), range.min)?;
    validate_positive_finite(format!("{path}.max"), range.max)?;
    if range.min > range.max {
        return Err(ProfileError::new(
            format!("{path}.max"),
            "must be greater than or equal to min",
        ));
    }
    Ok(())
}

fn validate_semver(path: &str, value: &str) -> Result<(), ProfileError> {
    let mut build_split = value.split('+');
    let core_and_pre = build_split.next().unwrap_or_default();
    let build = build_split.next();
    let has_extra_build_separator = build_split.next().is_some();
    let (core, pre) = core_and_pre
        .split_once('-')
        .map_or((core_and_pre, None), |(left, right)| (left, Some(right)));
    let parts: Vec<_> = core.split('.').collect();
    let valid_number = |part: &str| {
        !part.is_empty()
            && part.chars().all(|character| character.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    };
    let valid_identifiers = |identifiers: &str, reject_numeric_leading_zero: bool| {
        !identifiers.is_empty()
            && identifiers.split('.').all(|identifier| {
                !identifier.is_empty()
                    && identifier
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '-')
                    && (!reject_numeric_leading_zero
                        || !identifier
                            .chars()
                            .all(|character| character.is_ascii_digit())
                        || identifier == "0"
                        || !identifier.starts_with('0'))
            })
    };
    let valid_pre = pre.is_none_or(|pre| valid_identifiers(pre, true));
    let valid_build = build.is_none_or(|build| valid_identifiers(build, false));
    if parts.len() != 3
        || !parts.iter().all(|part| valid_number(part))
        || !valid_pre
        || !valid_build
        || has_extra_build_separator
    {
        return Err(ProfileError::new(path, "must be a semantic version"));
    }
    Ok(())
}

fn validate_rfc3339(path: &str, value: &str) -> Result<(), ProfileError> {
    let bytes = value.as_bytes();
    let digits = |start: usize, end: usize| {
        bytes
            .get(start..end)
            .filter(|slice| slice.iter().all(u8::is_ascii_digit))
            .and_then(|slice| std::str::from_utf8(slice).ok())
            .and_then(|slice| slice.parse::<u32>().ok())
    };
    let shape = bytes.len() >= 20
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && matches!(bytes.get(10), Some(b'T' | b't'))
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':');
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        digits(0, 4),
        digits(5, 7),
        digits(8, 10),
        digits(11, 13),
        digits(14, 16),
        digits(17, 19),
    ) else {
        return Err(ProfileError::new(path, "must be an RFC 3339 timestamp"));
    };
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => 0,
    };

    let mut timezone_index = 19;
    if bytes.get(timezone_index) == Some(&b'.') {
        timezone_index += 1;
        let fraction_start = timezone_index;
        while bytes.get(timezone_index).is_some_and(u8::is_ascii_digit) {
            timezone_index += 1;
        }
        if timezone_index == fraction_start {
            return Err(ProfileError::new(path, "must be an RFC 3339 timestamp"));
        }
    }
    let timezone_is_utc = bytes.get(timezone_index..timezone_index + 1) == Some(b"Z")
        || bytes.get(timezone_index..timezone_index + 1) == Some(b"z");
    let timezone_is_offset = if matches!(bytes.get(timezone_index), Some(b'+' | b'-')) {
        let offset_hour = digits(timezone_index + 1, timezone_index + 3);
        let offset_minute = digits(timezone_index + 4, timezone_index + 6);
        bytes.get(timezone_index + 3) == Some(&b':')
            && offset_hour.is_some_and(|hour| hour <= 23)
            && offset_minute.is_some_and(|minute| minute <= 59)
            && timezone_index + 6 == bytes.len()
    } else {
        false
    };
    let valid = shape
        && month > 0
        && day > 0
        && day <= days_in_month
        && hour <= 23
        && minute <= 59
        && second <= 60
        && ((timezone_is_utc && timezone_index + 1 == bytes.len()) || timezone_is_offset);
    if !valid {
        return Err(ProfileError::new(path, "must be an RFC 3339 timestamp"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineIntent {
    PhysicalCapture,
    Emulation,
    PostProduction,
}

/// Distinguishes measured hardware from a creative simulation or a signal transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Observed,
    Simulated,
    Transform,
}

/// Signal domains make physically invalid node connections detectable before rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDomain {
    SceneLight,
    OpticalImage,
    FilmLatentImage,
    FilmDensity,
    SensorRaw,
    SceneLinear,
    DisplayLinear,
    DisplayEncoded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagingPipeline {
    pub id: String,
    pub schema_version: u32,
    pub intent: PipelineIntent,
    pub working_color_space: WorkingColorSpace,
    pub nodes: Vec<PipelineNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineNode {
    pub id: String,
    pub role: NodeRole,
    pub enabled: bool,
    pub operation: ImagingOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "snake_case")]
pub enum ImagingOperation {
    Source(SourceNode),
    Camera(CameraNode),
    Lens(LensNode),
    CaptureMedium(CaptureMediumNode),
    Development(DevelopmentNode),
    Print(PrintNode),
    OutputTransform(OutputTransformNode),
    Display(DisplayNode),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceNode {
    pub source: SourceKind,
    pub output_domain: SignalDomain,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Scene,
    LiveCamera,
    ImageFile,
    VideoFile,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraNode {
    pub profile_id: String,
    pub shutter_seconds: f64,
    pub exposure_compensation_ev: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensNode {
    pub profile_id: String,
    pub focal_length_mm: f32,
    pub aperture_f_number: f32,
    pub focus_distance_m: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureMediumNode {
    pub medium: CaptureMedium,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureMedium {
    Film {
        profile_id: String,
        exposure_index: f32,
    },
    DigitalSensor {
        profile_id: String,
        iso: f32,
        bit_depth: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentNode {
    pub process: DevelopmentProcess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DevelopmentProcess {
    Chemical {
        process_id: String,
        push_pull_stops: f32,
    },
    DigitalRaw {
        profile_id: String,
        exposure_ev: f32,
        white_balance_kelvin: u32,
        tint: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintNode {
    pub process: PrintProcess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PrintProcess {
    Photochemical { profile_id: String },
    DigitalIntermediate { profile_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputTransformNode {
    pub transform_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayNode {
    pub profile_id: String,
    pub peak_luminance_nits: f32,
    pub surround: DisplaySurround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplaySurround {
    Dark,
    Dim,
    Average,
}

impl ImagingOperation {
    pub fn input_domain(&self) -> Option<SignalDomain> {
        match self {
            Self::Source(_) => None,
            Self::Camera(_) => Some(SignalDomain::SceneLight),
            Self::Lens(_) => Some(SignalDomain::SceneLight),
            Self::CaptureMedium(_) => Some(SignalDomain::OpticalImage),
            Self::Development(node) => Some(match node.process {
                DevelopmentProcess::Chemical { .. } => SignalDomain::FilmLatentImage,
                DevelopmentProcess::DigitalRaw { .. } => SignalDomain::SensorRaw,
            }),
            Self::Print(node) => Some(match node.process {
                PrintProcess::Photochemical { .. } => SignalDomain::FilmDensity,
                PrintProcess::DigitalIntermediate { .. } => SignalDomain::SceneLinear,
            }),
            Self::OutputTransform(_) => Some(SignalDomain::SceneLinear),
            Self::Display(_) => Some(SignalDomain::DisplayLinear),
        }
    }

    pub fn output_domain(&self) -> SignalDomain {
        match self {
            Self::Source(node) => node.output_domain,
            Self::Camera(_) => SignalDomain::SceneLight,
            Self::Lens(_) => SignalDomain::OpticalImage,
            Self::CaptureMedium(node) => match node.medium {
                CaptureMedium::Film { .. } => SignalDomain::FilmLatentImage,
                CaptureMedium::DigitalSensor { .. } => SignalDomain::SensorRaw,
            },
            Self::Development(node) => match node.process {
                DevelopmentProcess::Chemical { .. } => SignalDomain::FilmDensity,
                DevelopmentProcess::DigitalRaw { .. } => SignalDomain::SceneLinear,
            },
            Self::Print(_) | Self::OutputTransform(_) => SignalDomain::DisplayLinear,
            Self::Display(_) => SignalDomain::DisplayEncoded,
        }
    }
}

impl ImagingPipeline {
    pub fn validate(&self) -> Result<(), PipelineError> {
        if self.schema_version == 0 {
            return Err(PipelineError::InvalidSchemaVersion);
        }
        if self.nodes.is_empty() {
            return Err(PipelineError::Empty);
        }

        let enabled: Vec<_> = self.nodes.iter().filter(|node| node.enabled).collect();
        if enabled.is_empty() {
            return Err(PipelineError::Empty);
        }
        if !matches!(enabled[0].operation, ImagingOperation::Source(_)) {
            return Err(PipelineError::SourceMustBeFirst);
        }
        if enabled
            .iter()
            .skip(1)
            .any(|node| matches!(node.operation, ImagingOperation::Source(_)))
        {
            return Err(PipelineError::MultipleSources);
        }

        let mut ids = HashSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() || !ids.insert(node.id.as_str()) {
                return Err(PipelineError::InvalidNodeId(node.id.clone()));
            }
        }

        let mut domain = enabled[0].operation.output_domain();
        for node in enabled.iter().skip(1) {
            let expected = node
                .operation
                .input_domain()
                .expect("only the first enabled node may be a source");
            if domain != expected {
                return Err(PipelineError::DomainMismatch {
                    node_id: node.id.clone(),
                    expected,
                    actual: domain,
                });
            }
            domain = node.operation.output_domain();
        }

        if domain != SignalDomain::DisplayEncoded {
            return Err(PipelineError::Incomplete {
                final_domain: domain,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    Empty,
    InvalidSchemaVersion,
    InvalidNodeId(String),
    SourceMustBeFirst,
    MultipleSources,
    DomainMismatch {
        node_id: String,
        expected: SignalDomain,
        actual: SignalDomain,
    },
    Incomplete {
        final_domain: SignalDomain,
    },
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PipelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, role: NodeRole, operation: ImagingOperation) -> PipelineNode {
        PipelineNode {
            id: id.into(),
            role,
            enabled: true,
            operation,
        }
    }

    fn base(nodes: Vec<PipelineNode>) -> ImagingPipeline {
        ImagingPipeline {
            id: "test-pipeline".into(),
            schema_version: 1,
            intent: PipelineIntent::Emulation,
            working_color_space: WorkingColorSpace::AcesCg,
            nodes,
        }
    }

    fn source() -> PipelineNode {
        node(
            "scene",
            NodeRole::Observed,
            ImagingOperation::Source(SourceNode {
                source: SourceKind::Scene,
                output_domain: SignalDomain::SceneLight,
                profile_id: None,
            }),
        )
    }

    fn lens() -> PipelineNode {
        node(
            "lens",
            NodeRole::Simulated,
            ImagingOperation::Lens(LensNode {
                profile_id: "reference-lens".into(),
                focal_length_mm: 50.0,
                aperture_f_number: 2.8,
                focus_distance_m: Some(3.0),
            }),
        )
    }

    fn camera() -> PipelineNode {
        node(
            "camera",
            NodeRole::Observed,
            ImagingOperation::Camera(CameraNode {
                profile_id: "reference-camera-body".into(),
                shutter_seconds: 1.0 / 48.0,
                exposure_compensation_ev: 0.0,
            }),
        )
    }

    fn display() -> PipelineNode {
        node(
            "display",
            NodeRole::Transform,
            ImagingOperation::Display(DisplayNode {
                profile_id: "rec709-reference".into(),
                peak_luminance_nits: 100.0,
                surround: DisplaySurround::Dim,
            }),
        )
    }

    #[test]
    fn validates_complete_film_chain() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "negative",
                NodeRole::Simulated,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::Film {
                        profile_id: "synthetic-negative-500".into(),
                        exposure_index: 500.0,
                    },
                }),
            ),
            node(
                "development",
                NodeRole::Simulated,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::Chemical {
                        process_id: "ecn2-reference".into(),
                        push_pull_stops: 0.0,
                    },
                }),
            ),
            node(
                "print",
                NodeRole::Simulated,
                ImagingOperation::Print(PrintNode {
                    process: PrintProcess::Photochemical {
                        profile_id: "synthetic-print".into(),
                    },
                }),
            ),
            display(),
        ]);
        assert_eq!(pipeline.validate(), Ok(()));
    }

    #[test]
    fn validates_complete_digital_chain() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "sensor",
                NodeRole::Simulated,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::DigitalSensor {
                        profile_id: "reference-bayer-sensor".into(),
                        iso: 400.0,
                        bit_depth: 14,
                    },
                }),
            ),
            node(
                "raw-development",
                NodeRole::Transform,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::DigitalRaw {
                        profile_id: "neutral-raw".into(),
                        exposure_ev: 0.0,
                        white_balance_kelvin: 5600,
                        tint: 0.0,
                    },
                }),
            ),
            node(
                "output-transform",
                NodeRole::Transform,
                ImagingOperation::OutputTransform(OutputTransformNode {
                    transform_id: "aces-2-rec709".into(),
                }),
            ),
            display(),
        ]);
        assert_eq!(pipeline.validate(), Ok(()));
    }

    #[test]
    fn rejects_film_development_after_digital_sensor() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "sensor",
                NodeRole::Observed,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::DigitalSensor {
                        profile_id: "sensor".into(),
                        iso: 100.0,
                        bit_depth: 12,
                    },
                }),
            ),
            node(
                "wrong-development",
                NodeRole::Transform,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::Chemical {
                        process_id: "ecn2".into(),
                        push_pull_stops: 0.0,
                    },
                }),
            ),
            display(),
        ]);
        assert!(matches!(
            pipeline.validate(),
            Err(PipelineError::DomainMismatch { .. })
        ));
    }

    #[test]
    fn disabled_nodes_are_excluded_from_domain_flow() {
        let mut skipped_lens = lens();
        skipped_lens.enabled = false;
        let pipeline = base(vec![source(), skipped_lens, display()]);
        assert!(matches!(
            pipeline.validate(),
            Err(PipelineError::DomainMismatch { .. })
        ));
    }

    #[test]
    fn bundled_json_examples_match_the_contract() {
        let examples = [
            include_str!("../../../examples/pipelines/digital-reference.json"),
            include_str!("../../../examples/pipelines/film-reference.json"),
        ];
        for json in examples {
            let pipeline: ImagingPipeline = serde_json::from_str(json).unwrap();
            pipeline.validate().unwrap();
        }
    }

    #[test]
    fn bundled_profile_matches_common_contract_and_preserves_extensions() {
        let common_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/profile-common-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(common_schema["properties"]["schema_version"]["const"], 1);
        let film_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/film-profile-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            film_schema["allOf"][1]["properties"]["kind"]["const"],
            "film"
        );

        let json = include_str!("../../../examples/profiles/synthetic-color-negative-500.json");
        let profile = ProfileEnvelope::from_json(json).unwrap();
        assert_eq!(profile.kind, ProfileKind::Film);
        let film = profile.film_data().unwrap();
        assert_eq!(film.film_type, FilmType::ColorNegative);
        assert_eq!(film.sensitometry.samples.len(), 3);
        assert!(profile.extensions.contains_key("$schema"));
        assert!(profile.extensions.contains_key("semantic"));

        let round_trip = serde_json::to_string(&profile).unwrap();
        let decoded = ProfileEnvelope::from_json(&round_trip).unwrap();
        assert_eq!(decoded.extensions, profile.extensions);
    }

    #[test]
    fn film_profile_rejects_non_monotonic_sensitometry() {
        let json = include_str!("../../../examples/profiles/synthetic-color-negative-500.json");
        let mut profile = ProfileEnvelope::from_json(json).unwrap();
        profile.data["sensitometry"]["samples"][1]["log_exposure"] = (-4.0).into();
        let error = profile.film_data().unwrap_err();
        assert_eq!(error.path, "$.data.sensitometry.samples[1].log_exposure");
    }

    #[test]
    fn bundled_lens_and_sensor_profiles_match_typed_contracts() {
        let lens_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/lens-profile-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            lens_schema["allOf"][1]["properties"]["kind"]["const"],
            "lens"
        );
        let sensor_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/digital-sensor-profile-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            sensor_schema["allOf"][1]["properties"]["kind"]["const"],
            "digital_sensor"
        );

        let lens = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/reference-lens-50mm.json"
        ))
        .unwrap();
        let lens_data = lens.lens_data().unwrap();
        assert_eq!(lens_data.focal_length_mm.min, 50.0);
        assert_eq!(lens_data.focal_length_mm.max, 50.0);

        let sensor = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/reference-bayer-sensor.json"
        ))
        .unwrap();
        let sensor_data = sensor.digital_sensor_data().unwrap();
        assert_eq!(sensor_data.cfa, SensorCfa::BayerRggb);
        assert_eq!(sensor_data.native_bit_depth, 14);
        assert_eq!(sensor_data.spectral_sensitivity.len(), 3);
    }

    #[test]
    fn lens_profile_rejects_reversed_focal_range() {
        let json = include_str!("../../../examples/profiles/reference-lens-50mm.json");
        let mut profile: ProfileEnvelope = serde_json::from_str(json).unwrap();
        profile.data["focal_length_mm"]["min"] = 85.0.into();
        let error = profile.validate().unwrap_err();
        assert_eq!(error.path, "$.data.focal_length_mm.max");
    }

    #[test]
    fn sensor_profile_rejects_non_monotonic_spectral_wavelengths() {
        let json = include_str!("../../../examples/profiles/reference-bayer-sensor.json");
        let mut profile: ProfileEnvelope = serde_json::from_str(json).unwrap();
        profile.data["spectral_sensitivity"][1]["wavelength_nm"] = 400.into();
        let error = profile.validate().unwrap_err();
        assert_eq!(error.path, "$.data.spectral_sensitivity[1].wavelength_nm");
    }

    #[test]
    fn profile_validation_reports_a_json_path() {
        let json = include_str!("../../../examples/profiles/synthetic-color-negative-500.json");
        let mut profile: ProfileEnvelope = serde_json::from_str(json).unwrap();
        profile.profile_version = "version one".into();
        let error = profile.validate().unwrap_err();
        assert_eq!(error.path, "$.profile_version");
    }

    #[test]
    fn catalog_validates_reference_existence_and_kind() {
        let json = include_str!("../../../examples/profiles/synthetic-color-negative-500.json");
        let film = ProfileEnvelope::from_json(json).unwrap();
        let mut development = film.clone();
        development.id = "org.universal-imaging.synthetic-development".into();
        development.kind = ProfileKind::Development;
        development.references = vec![ProfileReference {
            profile_id: film.id.clone(),
            expected_kind: Some(ProfileKind::Film),
        }];

        let mut catalog = ProfileCatalog::default();
        catalog.insert(development.clone()).unwrap();
        let missing = catalog.validate_references().unwrap_err();
        assert!(missing.path.ends_with(".profile_id"));

        catalog.insert(film).unwrap();
        catalog.validate_references().unwrap();

        development.references[0].expected_kind = Some(ProfileKind::Lens);
        let mut wrong_kind = ProfileCatalog::default();
        wrong_kind.insert(development).unwrap();
        let mut film_again = ProfileEnvelope::from_json(json).unwrap();
        film_again.extensions.clear();
        wrong_kind.insert(film_again).unwrap();
        let mismatch = wrong_kind.validate_references().unwrap_err();
        assert!(mismatch.path.ends_with(".expected_kind"));
    }
}
