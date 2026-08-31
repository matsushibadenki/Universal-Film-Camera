//! Platform-neutral media contracts. Native camera handles stay behind this crate.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    Rgba16Float,
    Rgba32Float,
    Nv12,
    P010,
    Bgra8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingColorSpace {
    Aces2065,
    AcesCg,
    LinearRec2020,
    LinearP3,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferFunction {
    Linear,
    Srgb,
    Rec709,
    Pq,
    Hlg,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameDescriptor {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub color_space: WorkingColorSpace,
    pub transfer_function: TransferFunction,
}

#[derive(Debug)]
pub enum FrameStorage {
    Cpu(Vec<u8>),
    /// Opaque, process-local native texture/surface handle; never serialize over IPC.
    NativeHandle {
        handle: u64,
    },
}

#[derive(Debug)]
pub struct VideoFrame {
    pub timestamp_ns: u64,
    pub duration_ns: u64,
    pub descriptor: FrameDescriptor,
    pub storage: FrameStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorScopes {
    pub rgb_histogram: [[u32; 256]; 3],
    /// Luma distribution for each of 256 normalized horizontal positions.
    pub waveform: Vec<[u32; 256]>,
    /// Rec.709 Cb/Cr density map, row-major 256×256.
    pub vectorscope: Vec<u32>,
}

/// Deterministic CPU monitor reference for BGRA8 preview frames. Native/GPU
/// renderers must conform to these binning and luma/chroma equations.
pub fn analyze_bgra8(
    pixels: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: usize,
) -> Result<MonitorScopes, &'static str> {
    if width == 0 || height == 0 || bytes_per_row < width as usize * 4 {
        return Err("invalid BGRA8 frame geometry");
    }
    let required = bytes_per_row
        .checked_mul(height as usize)
        .ok_or("BGRA8 frame size overflow")?;
    if pixels.len() < required {
        return Err("BGRA8 frame storage is truncated");
    }
    let mut scopes = MonitorScopes {
        rgb_histogram: [[0; 256]; 3],
        waveform: vec![[0; 256]; 256],
        vectorscope: vec![0; 256 * 256],
    };
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * bytes_per_row + x * 4;
            let b = pixels[offset] as f32;
            let g = pixels[offset + 1] as f32;
            let r = pixels[offset + 2] as f32;
            scopes.rgb_histogram[0][r as usize] += 1;
            scopes.rgb_histogram[1][g as usize] += 1;
            scopes.rgb_histogram[2][b as usize] += 1;
            let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b)
                .round()
                .clamp(0.0, 255.0) as usize;
            let column = x * 256 / width as usize;
            scopes.waveform[column][luma] += 1;
            let cb = (128.0 + (b - luma as f32) / 1.8556)
                .round()
                .clamp(0.0, 255.0) as usize;
            let cr = (128.0 + (r - luma as f32) / 1.5748)
                .round()
                .clamp(0.0, 255.0) as usize;
            scopes.vectorscope[cb * 256 + cr] += 1;
        }
    }
    Ok(scopes)
}

pub fn false_color_bgra8(
    pixels: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: usize,
) -> Result<Vec<u8>, &'static str> {
    analyze_bgra8(pixels, width, height, bytes_per_row)?;
    let mut output = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * bytes_per_row + x * 4;
            let b = pixels[offset] as f32;
            let g = pixels[offset + 1] as f32;
            let r = pixels[offset + 2] as f32;
            let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
            let (r, g, b) = match luma {
                value if value < 0.025 => (128, 0, 255),
                value if value < 0.20 => (0, 64, 255),
                value if value < 0.42 => (64, 192, 64),
                value if value < 0.60 => (160, 160, 160),
                value if value < 0.78 => (255, 180, 0),
                value if value < 0.95 => (255, 48, 0),
                _ => (255, 0, 255),
            };
            output.extend_from_slice(&[b, g, r, 255]);
        }
    }
    Ok(output)
}

/// Returns a one-byte edge mask. A threshold of 0 includes every non-flat
/// edge; 255 disables all peaking.
pub fn focus_peaking_bgra8(
    pixels: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: usize,
    threshold: u8,
) -> Result<Vec<u8>, &'static str> {
    analyze_bgra8(pixels, width, height, bytes_per_row)?;
    let mut luma = vec![0u8; width as usize * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let p = y * bytes_per_row + x * 4;
            luma[y * width as usize + x] = (0.2126 * pixels[p + 2] as f32
                + 0.7152 * pixels[p + 1] as f32
                + 0.0722 * pixels[p] as f32)
                .round() as u8;
        }
    }
    let mut mask = vec![0u8; luma.len()];
    if width < 3 || height < 3 || threshold == 255 {
        return Ok(mask);
    }
    let w = width as usize;
    for y in 1..height as usize - 1 {
        for x in 1..w - 1 {
            let gx = luma[y * w + x + 1].abs_diff(luma[y * w + x - 1]);
            let gy = luma[(y + 1) * w + x].abs_diff(luma[(y - 1) * w + x]);
            mask[y * w + x] = if gx.saturating_add(gy) > threshold {
                255
            } else {
                0
            };
        }
    }
    Ok(mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_reference_bins_rgb_luma_and_neutral_chroma() {
        let pixels = [0, 0, 255, 255, 255, 255, 255, 255];
        let scopes = analyze_bgra8(&pixels, 2, 1, 8).unwrap();
        assert_eq!(scopes.rgb_histogram[0][255], 2);
        assert_eq!(scopes.rgb_histogram[1][0], 1);
        assert_eq!(scopes.rgb_histogram[2][255], 1);
        assert_eq!(scopes.waveform.iter().flatten().sum::<u32>(), 2);
        assert_eq!(scopes.vectorscope.iter().sum::<u32>(), 2);
    }

    #[test]
    fn false_color_and_focus_outputs_have_frame_dimensions() {
        let mut pixels = vec![0u8; 4 * 4 * 4];
        for y in 0..4 {
            for x in 2..4 {
                pixels[(y * 4 + x) * 4 + 2] = 255;
            }
        }
        assert_eq!(false_color_bgra8(&pixels, 4, 4, 16).unwrap().len(), 64);
        let mask = focus_peaking_bgra8(&pixels, 4, 4, 16, 10).unwrap();
        assert!(mask.iter().any(|value| *value == 255));
    }
}
