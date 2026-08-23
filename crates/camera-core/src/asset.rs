use crate::{CameraError, CapturedMediaType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const CAPTURED_ASSET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetState {
    Incomplete,
    Finalized,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Warning,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RationalRate {
    pub numerator: u64,
    pub denominator: u64,
}

impl RationalRate {
    pub fn as_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedCaptureFormat {
    pub width: u32,
    pub height: u32,
    pub fps: RationalRate,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedColorMetadata {
    pub embedded_profile: Option<String>,
    pub color_primaries: Option<String>,
    pub transfer_characteristic: Option<String>,
    pub matrix_coefficients: Option<String>,
    pub full_range: Option<bool>,
}

impl CapturedColorMetadata {
    fn is_declared(&self) -> bool {
        self.embedded_profile.is_some()
            || self.color_primaries.is_some()
            || self.transfer_characteristic.is_some()
            || self.matrix_coefficients.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaResource {
    pub path: PathBuf,
    pub byte_length: u64,
    pub container: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub bit_depth: Option<u16>,
    /// EXIF orientation value (1–8). Video resources use 1 and carry their
    /// container transform in `rotation_degrees` / `mirrored`.
    pub orientation: u16,
    pub orientation_explicit: bool,
    pub rotation_degrees: u16,
    pub mirrored: bool,
    pub frame_rate: Option<RationalRate>,
    pub duration_ms: Option<u64>,
    pub audio_channels: Option<u16>,
    pub audio_sample_rate_hz: Option<u32>,
    pub color: CapturedColorMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureMetadata {
    pub device_id: String,
    pub selected_format: SelectedCaptureFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub status: ValidationStatus,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetValidation {
    pub status: ValidationStatus,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedAsset {
    pub schema_version: u32,
    pub id: String,
    pub media_type: CapturedMediaType,
    pub state: AssetState,
    pub original: MediaResource,
    pub derivatives: Vec<MediaResource>,
    pub capture: CaptureMetadata,
    pub validation: AssetValidation,
    pub created_at_utc: String,
}

impl CapturedAsset {
    pub fn from_probed_resource(
        id: String,
        media_type: CapturedMediaType,
        mut original: MediaResource,
        finalized_path: PathBuf,
        capture: CaptureMetadata,
    ) -> Result<Self, CameraError> {
        let validation = validate_resource(&original, media_type, &capture.selected_format);
        if validation.status == ValidationStatus::Failed {
            let failures = validation
                .checks
                .iter()
                .filter(|check| check.status == ValidationStatus::Failed)
                .map(|check| check.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CameraError(format!(
                "captured asset failed validation: {failures}"
            )));
        }
        original.path = finalized_path;
        Ok(Self {
            schema_version: CAPTURED_ASSET_SCHEMA_VERSION,
            id,
            media_type,
            state: AssetState::Finalized,
            original,
            derivatives: Vec::new(),
            capture,
            validation,
            created_at_utc: rfc3339_now(),
        })
    }
}

pub fn probe_media_resource(
    path: impl AsRef<Path>,
    media_type: CapturedMediaType,
) -> Result<MediaResource, CameraError> {
    let path = path.as_ref();
    let bytes = fs::read(path)
        .map_err(|error| CameraError(format!("failed to read captured asset: {error}")))?;
    if bytes.is_empty() {
        return Err(CameraError("captured asset is empty".into()));
    }
    match media_type {
        CapturedMediaType::Photo => probe_jpeg(path, &bytes),
        CapturedMediaType::Video => probe_isobmff(path, &bytes),
    }
}

fn validate_resource(
    resource: &MediaResource,
    media_type: CapturedMediaType,
    selected: &SelectedCaptureFormat,
) -> AssetValidation {
    let mut checks = Vec::new();
    let dimensions_match = (resource.pixel_width == selected.width
        && resource.pixel_height == selected.height)
        || (matches!(resource.rotation_degrees, 90 | 270)
            && resource.pixel_width == selected.height
            && resource.pixel_height == selected.width);
    checks.push(check(
        "pixel_dimensions",
        dimensions_match,
        format!("{}x{}", selected.width, selected.height),
        format!("{}x{}", resource.pixel_width, resource.pixel_height),
    ));
    checks.push(check(
        "container",
        matches!(
            (media_type, resource.container.as_str()),
            (CapturedMediaType::Photo, "jpeg") | (CapturedMediaType::Video, "quicktime" | "mp4")
        ),
        match media_type {
            CapturedMediaType::Photo => "jpeg",
            CapturedMediaType::Video => "quicktime/mp4",
        }
        .into(),
        resource.container.clone(),
    ));
    checks.push(check(
        "video_codec",
        media_type == CapturedMediaType::Photo || resource.video_codec.is_some(),
        (media_type == CapturedMediaType::Video)
            .then_some("declared")
            .unwrap_or("not_applicable")
            .into(),
        resource
            .video_codec
            .clone()
            .unwrap_or_else(|| "not_applicable".into()),
    ));
    if media_type == CapturedMediaType::Video {
        let actual = resource.frame_rate.as_ref().map(RationalRate::as_f64);
        let expected = selected.fps.as_f64();
        checks.push(check(
            "frame_rate",
            actual.is_some_and(|value| (value - expected).abs() <= 0.12),
            format!("{expected:.3}"),
            actual.map_or_else(|| "missing".into(), |value| format!("{value:.3}")),
        ));
        checks.push(check(
            "positive_duration",
            resource.duration_ms.is_some_and(|value| value > 0),
            ">0ms".into(),
            resource
                .duration_ms
                .map_or_else(|| "missing".into(), |value| format!("{value}ms")),
        ));
        checks.push(check(
            "audio_track",
            resource.audio_codec.is_some(),
            "declared".into(),
            resource
                .audio_codec
                .clone()
                .unwrap_or_else(|| "missing".into()),
        ));
    }
    checks.push(ValidationCheck {
        name: "color_metadata".into(),
        status: if resource.color.is_declared() {
            ValidationStatus::Passed
        } else {
            ValidationStatus::Warning
        },
        expected: Some("embedded_or_declared".into()),
        actual: Some(
            if resource.color.is_declared() {
                "declared"
            } else {
                "missing"
            }
            .into(),
        ),
    });
    let status = if checks
        .iter()
        .any(|check| check.status == ValidationStatus::Failed)
    {
        ValidationStatus::Failed
    } else if checks
        .iter()
        .any(|check| check.status == ValidationStatus::Warning)
    {
        ValidationStatus::Warning
    } else {
        ValidationStatus::Passed
    };
    AssetValidation { status, checks }
}

fn check(name: &str, passed: bool, expected: String, actual: String) -> ValidationCheck {
    ValidationCheck {
        name: name.into(),
        status: if passed {
            ValidationStatus::Passed
        } else {
            ValidationStatus::Failed
        },
        expected: Some(expected),
        actual: Some(actual),
    }
}

fn probe_jpeg(path: &Path, bytes: &[u8]) -> Result<MediaResource, CameraError> {
    if bytes.get(..2) != Some(&[0xff, 0xd8]) {
        return Err(CameraError("photo output is not a JPEG stream".into()));
    }
    let mut width = None;
    let mut height = None;
    let mut bit_depth = None;
    let mut orientation = None;
    let mut exif_color_space = None;
    let mut has_icc = false;
    let mut cursor = 2;
    while cursor + 4 <= bytes.len() {
        if bytes[cursor] != 0xff {
            cursor += 1;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor] == 0xff {
            cursor += 1;
        }
        let Some(&marker) = bytes.get(cursor) else {
            break;
        };
        cursor += 1;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        let length = read_be_u16(bytes, cursor)
            .ok_or_else(|| CameraError("truncated JPEG marker".into()))?
            as usize;
        if length < 2 || cursor + length > bytes.len() {
            return Err(CameraError("invalid JPEG marker length".into()));
        }
        let payload = &bytes[cursor + 2..cursor + length];
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
            && payload.len() >= 5
        {
            bit_depth = Some(payload[0] as u16);
            height = Some(u16::from_be_bytes([payload[1], payload[2]]) as u32);
            width = Some(u16::from_be_bytes([payload[3], payload[4]]) as u32);
        } else if marker == 0xe1 && payload.starts_with(b"Exif\0\0") {
            let (parsed_orientation, color_space) = parse_tiff(&payload[6..]);
            orientation = parsed_orientation.or(orientation);
            exif_color_space = color_space.or(exif_color_space);
        } else if marker == 0xe2 && payload.starts_with(b"ICC_PROFILE\0") {
            has_icc = true;
        }
        cursor += length;
    }
    let (width, height) = width
        .zip(height)
        .ok_or_else(|| CameraError("JPEG dimensions are missing".into()))?;
    let explicit = orientation.is_some();
    let orientation = orientation.unwrap_or(1);
    let (rotation_degrees, mirrored) = exif_transform(orientation);
    let embedded_profile = if has_icc {
        Some("icc".into())
    } else if exif_color_space == Some(1) {
        Some("srgb_exif".into())
    } else {
        None
    };
    Ok(MediaResource {
        path: path.to_path_buf(),
        byte_length: bytes.len() as u64,
        container: "jpeg".into(),
        video_codec: Some("jpeg".into()),
        audio_codec: None,
        pixel_width: width,
        pixel_height: height,
        bit_depth,
        orientation,
        orientation_explicit: explicit,
        rotation_degrees,
        mirrored,
        frame_rate: None,
        duration_ms: None,
        audio_channels: None,
        audio_sample_rate_hz: None,
        color: CapturedColorMetadata {
            embedded_profile,
            ..Default::default()
        },
    })
}

fn parse_tiff(bytes: &[u8]) -> (Option<u16>, Option<u16>) {
    let little = match bytes.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return (None, None),
    };
    if read_u16(bytes, 2, little) != Some(42) {
        return (None, None);
    }
    let Some(ifd0) = read_u32(bytes, 4, little).map(|value| value as usize) else {
        return (None, None);
    };
    let orientation = find_tiff_short(bytes, ifd0, 0x0112, little);
    let color = find_tiff_long(bytes, ifd0, 0x8769, little)
        .and_then(|offset| find_tiff_short(bytes, offset as usize, 0xa001, little));
    (orientation.filter(|value| (1..=8).contains(value)), color)
}

fn find_tiff_short(bytes: &[u8], ifd: usize, tag: u16, little: bool) -> Option<u16> {
    let count = read_u16(bytes, ifd, little)? as usize;
    (0..count).find_map(|index| {
        let entry = ifd + 2 + index * 12;
        (read_u16(bytes, entry, little)? == tag
            && read_u16(bytes, entry + 2, little)? == 3
            && read_u32(bytes, entry + 4, little)? == 1)
            .then(|| read_u16(bytes, entry + 8, little))?
    })
}

fn find_tiff_long(bytes: &[u8], ifd: usize, tag: u16, little: bool) -> Option<u32> {
    let count = read_u16(bytes, ifd, little)? as usize;
    (0..count).find_map(|index| {
        let entry = ifd + 2 + index * 12;
        (read_u16(bytes, entry, little)? == tag).then(|| read_u32(bytes, entry + 8, little))?
    })
}

fn exif_transform(orientation: u16) -> (u16, bool) {
    match orientation {
        2 => (0, true),
        3 => (180, false),
        4 => (180, true),
        5 => (90, true),
        6 => (90, false),
        7 => (270, true),
        8 => (270, false),
        _ => (0, false),
    }
}

#[derive(Default)]
struct MovieTrack {
    handler: [u8; 4],
    codec: Option<String>,
    width: u32,
    height: u32,
    timescale: u32,
    duration: u64,
    sample_count: u64,
    sample_duration: u64,
    rotation_degrees: u16,
    mirrored: bool,
    audio_channels: Option<u16>,
    audio_sample_rate_hz: Option<u32>,
    color: CapturedColorMetadata,
}

fn probe_isobmff(path: &Path, bytes: &[u8]) -> Result<MediaResource, CameraError> {
    let top = boxes(bytes, 0, bytes.len())?;
    let ftyp = top
        .iter()
        .find(|item| item.kind == *b"ftyp")
        .ok_or_else(|| CameraError("movie output has no ftyp box".into()))?;
    let brand = bytes
        .get(ftyp.content..ftyp.content + 4)
        .unwrap_or_default();
    let container = if matches!(brand, b"qt  ") {
        "quicktime"
    } else {
        "mp4"
    };
    let moov = top
        .iter()
        .find(|item| item.kind == *b"moov")
        .ok_or_else(|| CameraError("movie output is not finalized (moov missing)".into()))?;
    let movie_duration_ms = boxes(bytes, moov.content, moov.end)?
        .into_iter()
        .find(|item| item.kind == *b"mvhd")
        .and_then(|mvhd| media_header_duration_ms(bytes, mvhd));
    let mut tracks = Vec::new();
    for trak in boxes(bytes, moov.content, moov.end)?
        .into_iter()
        .filter(|item| item.kind == *b"trak")
    {
        tracks.push(parse_track(bytes, trak)?);
    }
    let video = tracks
        .iter()
        .find(|track| track.handler == *b"vide")
        .ok_or_else(|| CameraError("movie output has no video track".into()))?;
    let audio = tracks.iter().find(|track| track.handler == *b"soun");
    let frame_rate = (video.sample_count > 0 && video.sample_duration > 0 && video.timescale > 0)
        .then(|| {
            reduce_rate(
                video.sample_count * u64::from(video.timescale),
                video.sample_duration,
            )
        });
    let duration_ms = movie_duration_ms.or_else(|| {
        (video.timescale > 0 && video.duration > 0)
            .then(|| video.duration.saturating_mul(1000) / u64::from(video.timescale))
    });
    Ok(MediaResource {
        path: path.to_path_buf(),
        byte_length: bytes.len() as u64,
        container: container.into(),
        video_codec: video.codec.clone(),
        audio_codec: audio.and_then(|track| track.codec.clone()),
        pixel_width: video.width,
        pixel_height: video.height,
        bit_depth: None,
        orientation: 1,
        orientation_explicit: false,
        rotation_degrees: video.rotation_degrees,
        mirrored: video.mirrored,
        frame_rate,
        duration_ms,
        audio_channels: audio.and_then(|track| track.audio_channels),
        audio_sample_rate_hz: audio.and_then(|track| track.audio_sample_rate_hz),
        color: video.color.clone(),
    })
}

fn media_header_duration_ms(bytes: &[u8], header: IsoBox) -> Option<u64> {
    let version = *bytes.get(header.content)?;
    let offset = header.content + if version == 1 { 20 } else { 12 };
    let timescale = read_be_u32(bytes, offset)?;
    if timescale == 0 {
        return None;
    }
    let duration = if version == 1 {
        read_be_u64(bytes, offset + 4)?
    } else {
        u64::from(read_be_u32(bytes, offset + 4)?)
    };
    (duration > 0).then(|| duration.saturating_mul(1000) / u64::from(timescale))
}

#[derive(Clone, Copy)]
struct IsoBox {
    kind: [u8; 4],
    content: usize,
    end: usize,
}

fn boxes(bytes: &[u8], mut cursor: usize, end: usize) -> Result<Vec<IsoBox>, CameraError> {
    let mut result = Vec::new();
    while cursor + 8 <= end {
        let size32 = read_be_u32(bytes, cursor)
            .ok_or_else(|| CameraError("truncated movie box".into()))? as u64;
        let kind: [u8; 4] = bytes[cursor + 4..cursor + 8].try_into().unwrap();
        let (size, header) = if size32 == 1 {
            (
                read_be_u64(bytes, cursor + 8)
                    .ok_or_else(|| CameraError("truncated extended movie box".into()))?,
                16,
            )
        } else if size32 == 0 {
            ((end - cursor) as u64, 8)
        } else {
            (size32, 8)
        };
        let box_end = cursor
            .checked_add(size as usize)
            .filter(|value| *value <= end)
            .ok_or_else(|| CameraError("invalid movie box length".into()))?;
        if size < header as u64 {
            return Err(CameraError("invalid movie box header".into()));
        }
        result.push(IsoBox {
            kind,
            content: cursor + header,
            end: box_end,
        });
        cursor = box_end;
    }
    Ok(result)
}

fn parse_track(bytes: &[u8], trak: IsoBox) -> Result<MovieTrack, CameraError> {
    let children = boxes(bytes, trak.content, trak.end)?;
    let mut track = MovieTrack::default();
    if let Some(tkhd) = children.iter().find(|item| item.kind == *b"tkhd") {
        let version = *bytes.get(tkhd.content).unwrap_or(&0);
        let matrix = tkhd.content + if version == 1 { 52 } else { 40 };
        let dimensions = tkhd.content + if version == 1 { 88 } else { 76 };
        if let (Some(a), Some(b), Some(c), Some(d)) = (
            read_be_i32(bytes, matrix),
            read_be_i32(bytes, matrix + 4),
            read_be_i32(bytes, matrix + 12),
            read_be_i32(bytes, matrix + 16),
        ) {
            (track.rotation_degrees, track.mirrored) = matrix_transform(a, b, c, d);
        }
        track.width = read_be_u32(bytes, dimensions).unwrap_or(0) >> 16;
        track.height = read_be_u32(bytes, dimensions + 4).unwrap_or(0) >> 16;
    }
    let mdia = children
        .iter()
        .find(|item| item.kind == *b"mdia")
        .ok_or_else(|| CameraError("movie track has no mdia box".into()))?;
    let mdia_children = boxes(bytes, mdia.content, mdia.end)?;
    if let Some(mdhd) = mdia_children.iter().find(|item| item.kind == *b"mdhd") {
        let version = *bytes.get(mdhd.content).unwrap_or(&0);
        let offset = mdhd.content + if version == 1 { 20 } else { 12 };
        track.timescale = read_be_u32(bytes, offset).unwrap_or(0);
        track.duration = if version == 1 {
            read_be_u64(bytes, offset + 4).unwrap_or(0)
        } else {
            u64::from(read_be_u32(bytes, offset + 4).unwrap_or(0))
        };
    }
    if let Some(hdlr) = mdia_children.iter().find(|item| item.kind == *b"hdlr") {
        if let Some(value) = bytes.get(hdlr.content + 8..hdlr.content + 12) {
            track.handler.copy_from_slice(value);
        }
    }
    let minf = mdia_children.iter().find(|item| item.kind == *b"minf");
    let stbl = minf
        .and_then(|item| boxes(bytes, item.content, item.end).ok())
        .and_then(|items| items.into_iter().find(|item| item.kind == *b"stbl"));
    if let Some(stbl) = stbl {
        for item in boxes(bytes, stbl.content, stbl.end)? {
            if item.kind == *b"stsd" {
                parse_stsd(bytes, item, &mut track)?;
            }
            if item.kind == *b"stts" {
                parse_stts(bytes, item, &mut track);
            }
        }
    }
    Ok(track)
}

fn parse_stsd(bytes: &[u8], stsd: IsoBox, track: &mut MovieTrack) -> Result<(), CameraError> {
    let start = stsd.content + 8;
    if start + 8 > stsd.end {
        return Err(CameraError("movie stsd entry is truncated".into()));
    }
    let size = read_be_u32(bytes, start).unwrap_or(0) as usize;
    if size < 16 || start + size > stsd.end {
        return Err(CameraError("movie sample entry is invalid".into()));
    }
    let kind: [u8; 4] = bytes[start + 4..start + 8].try_into().unwrap();
    track.codec = Some(codec_name(kind));
    if track.handler == *b"vide" {
        track.width = u32::from(read_be_u16(bytes, start + 32).unwrap_or(track.width as u16));
        track.height = u32::from(read_be_u16(bytes, start + 34).unwrap_or(track.height as u16));
        if start + 86 <= start + size {
            for child in boxes(bytes, start + 86, start + size).unwrap_or_default() {
                if child.kind == *b"colr" {
                    track.color = parse_colr(bytes, child);
                }
            }
        }
    } else if track.handler == *b"soun" {
        track.audio_channels = read_be_u16(bytes, start + 24);
        track.audio_sample_rate_hz = read_be_u32(bytes, start + 32).map(|value| value >> 16);
    }
    Ok(())
}

fn parse_stts(bytes: &[u8], stts: IsoBox, track: &mut MovieTrack) {
    let count = read_be_u32(bytes, stts.content + 4).unwrap_or(0) as usize;
    for index in 0..count {
        let offset = stts.content + 8 + index * 8;
        let samples = u64::from(read_be_u32(bytes, offset).unwrap_or(0));
        let delta = u64::from(read_be_u32(bytes, offset + 4).unwrap_or(0));
        track.sample_count = track.sample_count.saturating_add(samples);
        track.sample_duration = track
            .sample_duration
            .saturating_add(samples.saturating_mul(delta));
    }
}

fn parse_colr(bytes: &[u8], colr: IsoBox) -> CapturedColorMetadata {
    let kind = bytes
        .get(colr.content..colr.content + 4)
        .unwrap_or_default();
    if !matches!(kind, b"nclc" | b"nclx") {
        return CapturedColorMetadata::default();
    }
    let primaries = read_be_u16(bytes, colr.content + 4).map(color_primaries);
    let transfer = read_be_u16(bytes, colr.content + 6).map(transfer_characteristic);
    let matrix = read_be_u16(bytes, colr.content + 8).map(matrix_coefficients);
    let full_range = (kind == b"nclx").then(|| {
        bytes
            .get(colr.content + 10)
            .is_some_and(|value| value & 0x80 != 0)
    });
    CapturedColorMetadata {
        embedded_profile: None,
        color_primaries: primaries,
        transfer_characteristic: transfer,
        matrix_coefficients: matrix,
        full_range,
    }
}

fn codec_name(kind: [u8; 4]) -> String {
    match &kind {
        b"avc1" | b"avc3" => "h264".into(),
        b"hvc1" | b"hev1" => "hevc".into(),
        b"mp4a" => "aac".into(),
        b"lpcm" | b"sowt" | b"twos" => "pcm".into(),
        _ => String::from_utf8_lossy(&kind).trim().to_owned(),
    }
}

fn color_primaries(value: u16) -> String {
    match value {
        1 => "bt709",
        9 => "bt2020",
        12 => "display_p3",
        _ => "unknown",
    }
    .into()
}
fn transfer_characteristic(value: u16) -> String {
    match value {
        1 => "bt709",
        13 => "srgb",
        16 => "pq",
        18 => "hlg",
        _ => "unknown",
    }
    .into()
}
fn matrix_coefficients(value: u16) -> String {
    match value {
        0 => "identity",
        1 => "bt709",
        6 => "smpte170m",
        9 => "bt2020_ncl",
        10 => "bt2020_cl",
        _ => "unknown",
    }
    .into()
}

fn matrix_transform(a: i32, b: i32, c: i32, d: i32) -> (u16, bool) {
    let unit = 1 << 16;
    let normalize = |value: i32| {
        if value.abs() < 256 {
            0
        } else if value > 0 {
            unit
        } else {
            -unit
        }
    };
    let (a, b, c, d) = (normalize(a), normalize(b), normalize(c), normalize(d));
    let rotation = match (a, b, c, d) {
        (0, x, y, 0) if x > 0 && y < 0 => 90,
        (x, 0, 0, y) if x < 0 && y < 0 => 180,
        (0, x, y, 0) if x < 0 && y > 0 => 270,
        _ => 0,
    };
    let determinant = i64::from(a) * i64::from(d) - i64::from(b) * i64::from(c);
    (rotation, determinant < 0)
}

fn reduce_rate(numerator: u64, denominator: u64) -> RationalRate {
    let divisor = gcd(numerator, denominator).max(1);
    RationalRate {
        numerator: numerator / divisor,
        denominator: denominator / divisor,
    }
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn read_be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn read_be_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn read_be_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_be_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}
fn read_u16(bytes: &[u8], offset: usize, little: bool) -> Option<u16> {
    let value: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(if little {
        u16::from_le_bytes(value)
    } else {
        u16::from_be_bytes(value)
    })
}
fn read_u32(bytes: &[u8], offset: usize, little: bool) -> Option<u32> {
    let value: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(if little {
        u32::from_le_bytes(value)
    } else {
        u32::from_be_bytes(value)
    })
}

fn rfc3339_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = duration.as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        seconds_of_day / 3600,
        seconds_of_day % 3600 / 60,
        seconds_of_day % 60,
        duration.subsec_millis()
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += (month <= 2) as i64;
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_probe_reads_dimensions_orientation_and_srgb() {
        let mut jpeg = vec![0xff, 0xd8];
        let exif = [
            b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 2, 0, 0x12, 0x01, 3, 0, 1,
            0, 0, 0, 6, 0, 0, 0, 0x69, 0x87, 4, 0, 1, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0x01,
            0xa0, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ];
        jpeg.extend([0xff, 0xe1]);
        jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
        jpeg.extend(exif);
        jpeg.extend([
            0xff, 0xc0, 0, 17, 8, 4, 0, 6, 0, 3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0, 0xff, 0xd9,
        ]);
        let resource = probe_jpeg(Path::new("fixture.jpg"), &jpeg).unwrap();
        assert_eq!((resource.pixel_width, resource.pixel_height), (1536, 1024));
        assert_eq!(resource.orientation, 6);
        assert_eq!(resource.rotation_degrees, 90);
        assert_eq!(
            resource.color.embedded_profile.as_deref(),
            Some("srgb_exif")
        );
    }

    #[test]
    fn selected_format_validation_rejects_wrong_dimensions() {
        let resource = MediaResource {
            path: "capture.jpg".into(),
            byte_length: 1,
            container: "jpeg".into(),
            video_codec: Some("jpeg".into()),
            audio_codec: None,
            pixel_width: 1920,
            pixel_height: 1080,
            bit_depth: Some(8),
            orientation: 1,
            orientation_explicit: false,
            rotation_degrees: 0,
            mirrored: false,
            frame_rate: None,
            duration_ms: None,
            audio_channels: None,
            audio_sample_rate_hz: None,
            color: CapturedColorMetadata {
                embedded_profile: Some("icc".into()),
                ..Default::default()
            },
        };
        let selected = SelectedCaptureFormat {
            width: 1280,
            height: 720,
            fps: RationalRate {
                numerator: 24,
                denominator: 1,
            },
        };
        assert_eq!(
            validate_resource(&resource, CapturedMediaType::Photo, &selected).status,
            ValidationStatus::Failed
        );
    }

    #[test]
    fn rate_is_reduced_without_losing_ntsc_fraction() {
        assert_eq!(
            reduce_rate(90_000, 3_003),
            RationalRate {
                numerator: 30_000,
                denominator: 1_001
            }
        );
    }

    #[test]
    fn movie_probe_reads_tracks_timing_and_color_metadata() {
        let mut movie_header = vec![0; 20];
        movie_header[12..16].copy_from_slice(&1000u32.to_be_bytes());
        movie_header[16..20].copy_from_slice(&5671u32.to_be_bytes());

        let mut track_header = vec![0; 84];
        track_header[40..44].copy_from_slice(&65536i32.to_be_bytes());
        track_header[56..60].copy_from_slice(&65536i32.to_be_bytes());
        track_header[72..76].copy_from_slice(&0x40000000i32.to_be_bytes());
        track_header[76..80].copy_from_slice(&(1920u32 << 16).to_be_bytes());
        track_header[80..84].copy_from_slice(&(1080u32 << 16).to_be_bytes());
        let video_trak = synthetic_track(
            b"vide",
            b"avc1",
            track_header,
            Some((1920, 1080)),
            600,
            3360,
            Some((168, 20)),
        );
        let audio_trak =
            synthetic_track(b"soun", b"mp4a", vec![0; 84], None, 48_000, 266_240, None);
        let mut moov_payload = make_box(b"mvhd", movie_header);
        moov_payload.extend(video_trak);
        moov_payload.extend(audio_trak);
        let mut movie = make_box(b"ftyp", b"qt  \0\0\0\0".to_vec());
        movie.extend(make_box(b"moov", moov_payload));

        let resource = probe_isobmff(Path::new("fixture.mov"), &movie).unwrap();
        assert_eq!(resource.container, "quicktime");
        assert_eq!(resource.video_codec.as_deref(), Some("h264"));
        assert_eq!(resource.audio_codec.as_deref(), Some("aac"));
        assert_eq!((resource.pixel_width, resource.pixel_height), (1920, 1080));
        assert_eq!(
            resource.frame_rate.unwrap(),
            RationalRate {
                numerator: 30,
                denominator: 1
            }
        );
        assert_eq!(resource.duration_ms, Some(5671));
        assert_eq!(resource.color.color_primaries.as_deref(), Some("bt709"));
        assert_eq!(resource.audio_channels, Some(1));
        assert_eq!(resource.audio_sample_rate_hz, Some(48_000));
    }

    #[test]
    fn unix_epoch_calendar_conversion_is_stable() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_324), (2025, 8, 24));
    }

    fn synthetic_track(
        handler: &[u8; 4],
        codec: &[u8; 4],
        track_header: Vec<u8>,
        dimensions: Option<(u16, u16)>,
        timescale: u32,
        duration: u32,
        timing: Option<(u32, u32)>,
    ) -> Vec<u8> {
        let mut media_header = vec![0; 20];
        media_header[12..16].copy_from_slice(&timescale.to_be_bytes());
        media_header[16..20].copy_from_slice(&duration.to_be_bytes());
        let mut handler_payload = vec![0; 12];
        handler_payload[8..12].copy_from_slice(handler);

        let mut sample_payload = if dimensions.is_some() {
            vec![0; 78]
        } else {
            vec![0; 28]
        };
        if let Some((width, height)) = dimensions {
            sample_payload[24..26].copy_from_slice(&width.to_be_bytes());
            sample_payload[26..28].copy_from_slice(&height.to_be_bytes());
            let mut color = b"nclc".to_vec();
            color.extend(1u16.to_be_bytes());
            color.extend(1u16.to_be_bytes());
            color.extend(1u16.to_be_bytes());
            sample_payload.extend(make_box(b"colr", color));
        } else {
            sample_payload[16..18].copy_from_slice(&1u16.to_be_bytes());
            sample_payload[24..28].copy_from_slice(&(48_000u32 << 16).to_be_bytes());
        }
        let sample_entry = make_box(codec, sample_payload);
        let mut sample_description = vec![0; 8];
        sample_description[4..8].copy_from_slice(&1u32.to_be_bytes());
        sample_description.extend(sample_entry);
        let mut sample_table = make_box(b"stsd", sample_description);
        if let Some((count, delta)) = timing {
            let mut time_to_sample = vec![0; 16];
            time_to_sample[4..8].copy_from_slice(&1u32.to_be_bytes());
            time_to_sample[8..12].copy_from_slice(&count.to_be_bytes());
            time_to_sample[12..16].copy_from_slice(&delta.to_be_bytes());
            sample_table.extend(make_box(b"stts", time_to_sample));
        }
        let minf = make_box(b"minf", make_box(b"stbl", sample_table));
        let mut mdia = make_box(b"mdhd", media_header);
        mdia.extend(make_box(b"hdlr", handler_payload));
        mdia.extend(minf);
        let mut trak = make_box(b"tkhd", track_header);
        trak.extend(make_box(b"mdia", mdia));
        make_box(b"trak", trak)
    }

    fn make_box(kind: &[u8; 4], payload: Vec<u8>) -> Vec<u8> {
        let mut result = Vec::with_capacity(payload.len() + 8);
        result.extend(((payload.len() + 8) as u32).to_be_bytes());
        result.extend(kind);
        result.extend(payload);
        result
    }
}
