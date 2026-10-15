use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LUT_BYTES: usize = 4 * 1024 * 1024;

// Same display-referred archetypes used by the monitor; not measured film stocks.
fn built_in_look(id: &str, rgb: [f32; 3]) -> [f32; 3] {
    if id == "none" {
        return rgb;
    }
    let mono = id.starts_with("bw-");
    let vivid = id.contains("vivid") || id.contains("hard") || id == "bleach-bypass";
    let soft = id.contains("soft") || id.contains("pastel") || id == "archive-faded";
    let warm = id.contains("warm") || id == "negative-tungsten";
    let cool = id.contains("cool");
    let contrast = if vivid {
        1.22
    } else if soft {
        0.84
    } else {
        1.05
    };
    let saturation = if mono {
        0.0
    } else if id == "bleach-bypass" {
        0.48
    } else if soft {
        0.82
    } else if vivid {
        1.18
    } else {
        1.02
    };
    let lift = if soft { 0.035 } else { 0.0 };
    let luma = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
    let gains = [
        if warm {
            1.055
        } else if cool {
            0.955
        } else {
            1.0
        },
        1.0,
        if warm {
            0.925
        } else if cool {
            1.06
        } else {
            1.0
        },
    ];
    if id == "bw-orthochromatic" {
        return [(rgb[0] * 0.15 + rgb[1] * 0.85).clamp(0., 1.); 3];
    }
    std::array::from_fn(|i| {
        (((luma + (rgb[i] - luma) * saturation - 0.5) * contrast + 0.5 + lift) * gains[i])
            .clamp(0., 1.)
    })
}

impl LutPayload {
    pub fn provenance(&self, id: &str) -> camera_core::AppliedLutMetadata {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(b"ufc-lut-red-fastest-f32-v1");
        hash.update((self.size as u32).to_le_bytes());
        for value in self
            .domain_min
            .iter()
            .chain(self.domain_max.iter())
            .chain(self.samples.iter().flatten())
        {
            hash.update(value.to_le_bytes());
        }
        camera_core::AppliedLutMetadata {
            id: id.into(),
            sha256: format!("{:x}", hash.finalize()),
            size: self.size,
            domain_min: self.domain_min.map(|v| v.to_string()),
            domain_max: self.domain_max.map(|v| v.to_string()),
            input_encoding: "srgb-display-referred".into(),
            output_encoding: "srgb".into(),
            renderer: "wgpu-cube-v1".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LutEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub source: String,
    pub size: usize,
}

#[derive(Debug, Serialize)]
pub struct LutCatalog {
    pub built_in: Vec<LutEntry>,
    pub imported: Vec<LutEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LutPayload {
    pub size: usize,
    pub samples: Vec<[f32; 3]>,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

const BUILT_INS: &[(&str, &str, &str)] = &[
    ("none", "Clean / No LUT", "neutral"),
    (
        "negative-daylight-soft",
        "Daylight Negative · Soft",
        "negative",
    ),
    (
        "negative-daylight-rich",
        "Daylight Negative · Rich",
        "negative",
    ),
    ("negative-tungsten", "Tungsten Negative", "negative"),
    ("negative-pastel", "Pastel Negative", "negative"),
    (
        "negative-warm-consumer",
        "Warm Consumer Negative",
        "negative",
    ),
    (
        "negative-cool-consumer",
        "Cool Consumer Negative",
        "negative",
    ),
    (
        "reversal-neutral",
        "Daylight Reversal · Neutral",
        "reversal",
    ),
    ("reversal-vivid", "Daylight Reversal · Vivid", "reversal"),
    ("reversal-warm", "Warm Reversal", "reversal"),
    ("print-warm", "Warm Release Print", "print"),
    ("print-cool", "Cool Release Print", "print"),
    ("bleach-bypass", "Bleach Bypass", "process"),
    ("archive-faded", "Faded Archive", "process"),
    (
        "bw-panchromatic-soft",
        "B&W Panchromatic · Soft",
        "monochrome",
    ),
    (
        "bw-panchromatic-hard",
        "B&W Panchromatic · Hard",
        "monochrome",
    ),
    ("bw-orthochromatic", "B&W Orthochromatic", "monochrome"),
];

pub fn catalog(directory: &Path) -> Result<LutCatalog, String> {
    let built_in = BUILT_INS
        .iter()
        .map(|(id, name, category)| LutEntry {
            id: (*id).into(),
            name: (*name).into(),
            category: (*category).into(),
            source: "built_in".into(),
            size: 33,
        })
        .collect();
    let mut imported = Vec::new();
    if directory.exists() {
        for entry in fs::read_dir(directory)
            .map_err(|error| format!("failed to read LUT directory: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("failed to read LUT entry: {error}"))?
                .path();
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_none_or(|value| !value.eq_ignore_ascii_case("cube"))
            {
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read imported LUT: {error}"))?;
            let (name, size) = validate_cube(&content)?;
            imported.push(LutEntry {
                id: format!(
                    "imported:{}",
                    path.file_stem().and_then(|v| v.to_str()).unwrap_or("lut")
                ),
                name,
                category: "imported".into(),
                source: "imported".into(),
                size,
            });
        }
    }
    imported.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(LutCatalog { built_in, imported })
}

pub fn import(directory: &Path, file_name: &str, content: &str) -> Result<LutEntry, String> {
    if !file_name.to_ascii_lowercase().ends_with(".cube") {
        return Err("only .cube LUT files are supported".into());
    }
    if content.len() > MAX_LUT_BYTES {
        return Err("LUT exceeds the 4 MiB import limit".into());
    }
    let (name, size) = validate_cube(content)?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create LUT directory: {error}"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let stem = file_name
        .trim_end_matches(|c: char| c != '.')
        .trim_end_matches('.')
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
        })
        .take(48)
        .collect::<String>();
    let safe_stem = if stem.is_empty() { "external" } else { &stem };
    let final_path = directory.join(format!("{safe_stem}-{stamp}.cube"));
    let partial_path = final_path.with_extension("cube.partial");
    let mut file = fs::File::create(&partial_path)
        .map_err(|error| format!("failed to create LUT: {error}"))?;
    file.write_all(content.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to persist LUT: {error}"))?;
    fs::rename(&partial_path, &final_path)
        .map_err(|error| format!("failed to finalize LUT: {error}"))?;
    Ok(LutEntry {
        id: format!(
            "imported:{}",
            final_path
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("lut")
        ),
        name,
        category: "imported".into(),
        source: "imported".into(),
        size,
    })
}

pub fn payload(directory: &Path, id: &str) -> Result<LutPayload, String> {
    if BUILT_INS.iter().any(|entry| entry.0 == id) {
        let size = if id == "none" { 2 } else { 33 };
        let mut samples = Vec::with_capacity(size * size * size);
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    samples.push(built_in_look(
                        id,
                        [
                            r as f32 / (size - 1) as f32,
                            g as f32 / (size - 1) as f32,
                            b as f32 / (size - 1) as f32,
                        ],
                    ));
                }
            }
        }
        return Ok(LutPayload {
            size,
            samples,
            domain_min: [0.; 3],
            domain_max: [1.; 3],
        });
    }
    let stem = id
        .strip_prefix("imported:")
        .ok_or("only imported LUTs have file payloads")?;
    if stem.is_empty()
        || !stem
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return Err("invalid imported LUT identifier".into());
    }
    let path = directory.join(format!("{stem}.cube"));
    let canonical_directory = fs::canonicalize(directory)
        .map_err(|error| format!("failed to resolve LUT directory: {error}"))?;
    let canonical_path = fs::canonicalize(&path)
        .map_err(|error| format!("failed to resolve imported LUT: {error}"))?;
    if !canonical_path.starts_with(&canonical_directory) || !canonical_path.is_file() {
        return Err("imported LUT is outside the managed directory".into());
    }
    if fs::metadata(&canonical_path)
        .map_err(|e| e.to_string())?
        .len()
        > MAX_LUT_BYTES as u64
    {
        return Err("LUT exceeds the 4 MiB import limit".into());
    }
    let content = fs::read_to_string(canonical_path)
        .map_err(|error| format!("failed to read imported LUT: {error}"))?;
    parse_cube(&content)
}

fn parse_cube(content: &str) -> Result<LutPayload, String> {
    let (_, size) = validate_cube(content)?;
    let mut samples = Vec::with_capacity(size.pow(3));
    let mut domain_min = [0.0; 3];
    let mut domain_max = [1.0; 3];
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("TITLE")
            || line.starts_with("LUT_3D_SIZE")
        {
            continue;
        }
        let numeric = line
            .strip_prefix("DOMAIN_MIN")
            .or_else(|| line.strip_prefix("DOMAIN_MAX"))
            .unwrap_or(line);
        let values = numeric
            .split_whitespace()
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "invalid LUT numeric value".to_string())?;
        if line.starts_with("DOMAIN_MIN") && values.len() == 3 {
            domain_min.copy_from_slice(&values);
        } else if line.starts_with("DOMAIN_MAX") && values.len() == 3 {
            domain_max.copy_from_slice(&values);
        } else if values.len() == 3 {
            samples.push([values[0], values[1], values[2]]);
        }
    }
    if domain_min
        .iter()
        .zip(domain_max)
        .any(|(min, max)| !min.is_finite() || !max.is_finite() || min >= &max)
    {
        return Err("LUT domain minimum must be lower than maximum".into());
    }
    Ok(LutPayload {
        size,
        samples,
        domain_min,
        domain_max,
    })
}

fn validate_cube(content: &str) -> Result<(String, usize), String> {
    if content.len() > MAX_LUT_BYTES {
        return Err("LUT exceeds the 4 MiB import limit".into());
    }
    let mut title = None;
    let mut size = None;
    let mut samples = 0usize;
    let mut domains = [None::<[f32; 3]>, None];
    for (index, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("TITLE") {
            let value = value.trim().trim_matches('"');
            if value.is_empty() || value.chars().count() > 120 {
                return Err("LUT TITLE must contain 1–120 characters".into());
            }
            title = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("LUT_3D_SIZE") {
            if size.is_some() || samples != 0 {
                return Err("duplicate or misplaced LUT_3D_SIZE".into());
            }
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("invalid LUT_3D_SIZE at line {}", index + 1))?;
            if !(2..=65).contains(&parsed) {
                return Err("LUT_3D_SIZE must be between 2 and 65".into());
            }
            size = Some(parsed);
        } else if line.starts_with("LUT_1D_SIZE") {
            return Err("1D LUT import is not supported".into());
        } else if line.starts_with("DOMAIN_MIN") || line.starts_with("DOMAIN_MAX") {
            let slot = usize::from(line.starts_with("DOMAIN_MAX"));
            let values = line
                .split_whitespace()
                .skip(1)
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid LUT domain")?;
            if domains[slot].is_some()
                || samples != 0
                || values.len() != 3
                || values.iter().any(|v| !v.is_finite())
            {
                return Err("invalid, duplicate or misplaced LUT domain".into());
            }
            domains[slot] = Some([values[0], values[1], values[2]]);
        } else {
            let values = line
                .split_whitespace()
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| format!("invalid LUT sample at line {}", index + 1))?;
            if values.len() != 3
                || values
                    .iter()
                    .any(|value| !value.is_finite() || !(-16.0..=16.0).contains(value))
            {
                return Err(format!("invalid RGB LUT sample at line {}", index + 1));
            }
            samples += 1;
        }
    }
    let min = domains[0].unwrap_or([0.; 3]);
    let max = domains[1].unwrap_or([1.; 3]);
    if (0..3).any(|i| min[i] >= max[i]) {
        return Err("LUT domain must be increasing".into());
    }
    let size = size.ok_or("LUT_3D_SIZE is required")?;
    let expected = size.checked_pow(3).ok_or("LUT size overflow")?;
    if samples != expected {
        return Err(format!(
            "LUT declares {size}³ but contains {samples} samples; expected {expected}"
        ));
    }
    Ok((title.unwrap_or_else(|| "Imported LUT".into()), size))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tables_have_red_fastest_order_and_content_hashes() {
        let mut lut = payload(Path::new("unused"), "none").unwrap();
        assert_eq!(lut.samples[1], [1., 0., 0.]);
        assert_eq!(lut.samples[4], [0., 0., 1.]);
        let original = lut.provenance("none");
        lut.samples[0][0] = 0.1;
        assert_ne!(original.sha256, lut.provenance("none").sha256);
        for &(id, _, _) in BUILT_INS {
            let lut = payload(Path::new("unused"), id).unwrap();
            assert_eq!(lut.samples.len(), lut.size.pow(3));
            assert!(
                lut.samples
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            );
        }
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(
            serde_json::from_str::<camera_core::AppliedLutMetadata>(&json).unwrap(),
            original
        );
    }
    #[test]
    fn rejects_nonfinite_and_duplicate_domains() {
        let samples = "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
        assert_eq!(parse_cube(samples).unwrap().samples[1], [1., 0., 0.]);
        for domain in [
            "DOMAIN_MIN NaN 0 0",
            "DOMAIN_MAX inf 1 1",
            "DOMAIN_MIN 0 0",
            "DOMAIN_MIN 1 0 0",
            "DOMAIN_MIN 0 0 0\nDOMAIN_MIN 0 0 0",
        ] {
            assert!(parse_cube(&format!("{domain}\n{samples}")).is_err());
        }
    }
    #[test]
    fn validates_complete_cube_and_rejects_truncation() {
        let valid = "TITLE \"Test\"\nLUT_3D_SIZE 2\n0 0 0\n0 0 1\n0 1 0\n0 1 1\n1 0 0\n1 0 1\n1 1 0\n1 1 1\n";
        assert_eq!(validate_cube(valid).unwrap(), ("Test".into(), 2));
        assert!(validate_cube("LUT_3D_SIZE 2\n0 0 0\n").is_err());
        let payload = parse_cube(valid).unwrap();
        assert_eq!(payload.size, 2);
        assert_eq!(payload.samples.len(), 8);
        assert_eq!(payload.samples[7], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn import_stays_inside_managed_directory_and_is_listed() {
        let directory = std::env::temp_dir().join(format!("ufc-lut-test-{}", std::process::id()));
        let valid = "TITLE \"External Test\"\nLUT_3D_SIZE 2\n0 0 0\n0 0 1\n0 1 0\n0 1 1\n1 0 0\n1 0 1\n1 1 0\n1 1 1\n";
        let imported = import(&directory, "../../unsafe.cube", valid).unwrap();
        assert_eq!(imported.name, "External Test");
        assert_eq!(catalog(&directory).unwrap().imported.len(), 1);
        assert!(
            directory
                .read_dir()
                .unwrap()
                .all(|entry| entry.unwrap().path().parent() == Some(directory.as_path()))
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
