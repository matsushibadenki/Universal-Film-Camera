use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LUT_BYTES: usize = 4 * 1024 * 1024;

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

#[derive(Debug, Serialize)]
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
        .any(|(min, max)| min >= &max)
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
            continue;
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
    use super::{catalog, import, parse_cube, validate_cube};
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
