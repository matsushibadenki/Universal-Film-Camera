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
