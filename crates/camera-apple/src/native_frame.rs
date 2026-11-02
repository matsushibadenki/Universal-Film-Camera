//! Native camera input boundary. No pixels are copied or serialized here.
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CFRetained;
use objc2_core_video::{
    CVMetalTexture, CVMetalTextureCache, CVMetalTextureGetTexture, CVPixelBuffer,
    CVPixelBufferGetHeight, CVPixelBufferGetHeightOfPlane, CVPixelBufferGetPixelFormatType,
    CVPixelBufferGetWidth, CVPixelBufferGetWidthOfPlane, kCVPixelFormatType_32BGRA,
    kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
    kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
};
use objc2_metal::{MTLDevice, MTLPixelFormat};
use std::{
    ptr::NonNull,
    sync::{Arc, Mutex},
};

pub struct NativeFrame {
    buffer: CFRetained<CVPixelBuffer>,
    pub sequence: u64,
}
// SAFETY: AVCapture owns the producer; after delivery the retained image is
// read-only to this consumer. CoreVideo retain/release supports cross-thread use.
// Keeping the CVPixelBuffer retained prevents pool reuse while a lease exists.
unsafe impl Send for NativeFrame {}
unsafe impl Sync for NativeFrame {}

impl NativeFrame {
    /// SDR input contract for the GPU preview shader. Unsupported/missing YUV
    /// color metadata must fall back rather than silently guess a color matrix.
    pub fn preview_color(&self) -> Result<[u32; 4], String> {
        use objc2_core_foundation::CFString;
        use objc2_core_video::*;
        let matches = |key: &CFString, expected: &CFString| {
            unsafe { self.buffer.attachment(key, std::ptr::null_mut()) }
                .and_then(|value| value.downcast_ref::<CFString>().map(|s| s == expected))
                .unwrap_or(false)
        };
        if !matches(unsafe { kCVImageBufferColorPrimariesKey }, unsafe {
            kCVImageBufferColorPrimaries_ITU_R_709_2
        }) {
            return Err("native preview needs Rec.709 SDR primaries".into());
        }
        let matrix = if self.pixel_format() == kCVPixelFormatType_32BGRA {
            0
        } else if matches(unsafe { kCVImageBufferYCbCrMatrixKey }, unsafe {
            kCVImageBufferYCbCrMatrix_ITU_R_709_2
        }) {
            1
        } else if matches(unsafe { kCVImageBufferYCbCrMatrixKey }, unsafe {
            kCVImageBufferYCbCrMatrix_ITU_R_601_4
        }) {
            2
        } else {
            return Err("unsupported or missing YUV matrix".into());
        };
        let transfer = if matches(unsafe { kCVImageBufferTransferFunctionKey }, unsafe {
            kCVImageBufferTransferFunction_ITU_R_709_2
        }) {
            1
        } else if matches(unsafe { kCVImageBufferTransferFunctionKey }, unsafe {
            kCVImageBufferTransferFunction_sRGB
        }) {
            0
        } else {
            return Err("unsupported or missing SDR transfer".into());
        };
        if matrix == 0 {
            return Ok([0, 0, transfer, 0]);
        }
        let left = if matches(unsafe { kCVImageBufferChromaLocationTopFieldKey }, unsafe {
            kCVImageBufferChromaLocation_Left
        }) {
            1
        } else if matches(unsafe { kCVImageBufferChromaLocationTopFieldKey }, unsafe {
            kCVImageBufferChromaLocation_Center
        }) {
            0
        } else {
            return Err("unsupported or missing chroma location".into());
        };
        Ok([
            matrix,
            u32::from(self.pixel_format() == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange),
            transfer,
            left,
        ])
    }
    pub fn width(&self) -> usize {
        CVPixelBufferGetWidth(&self.buffer)
    }
    pub fn height(&self) -> usize {
        CVPixelBufferGetHeight(&self.buffer)
    }
    pub fn pixel_format(&self) -> u32 {
        CVPixelBufferGetPixelFormatType(&self.buffer)
    }
}

#[derive(Default)]
struct Mailbox {
    latest: Option<Arc<NativeFrame>>,
    received: u64,
    replaced: u64,
    cpu_rgb_generated: u64,
}

/// Capacity one: a slow renderer receives the newest image, never a growing queue.
#[derive(Default)]
pub struct NativeFrameMailbox(Mutex<Mailbox>);

impl NativeFrameMailbox {
    pub fn publish(&self, buffer: CFRetained<CVPixelBuffer>) {
        if let Ok(mut state) = self.0.lock() {
            state.received = state.received.saturating_add(1);
            if state.latest.is_some() {
                state.replaced = state.replaced.saturating_add(1);
            }
            state.latest = Some(Arc::new(NativeFrame {
                buffer,
                sequence: state.received,
            }));
        }
    }
    pub fn take_latest(&self) -> Option<Arc<NativeFrame>> {
        self.0.lock().ok()?.latest.take()
    }
    pub fn clear(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.latest = None;
        }
    }
    pub(crate) fn record_cpu_rgb(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.cpu_rgb_generated = state.cpu_rgb_generated.saturating_add(1);
        }
    }
    pub fn counters(&self) -> (u64, u64, u64) {
        self.0
            .lock()
            .map(|state| (state.received, state.replaced, state.cpu_rgb_generated))
            .unwrap_or_default()
    }
}

/// Retain the CVMetalTexture wrappers AND source image until GPU work completes.
/// A retained MTLTexture alone is not sufficient to prevent CoreVideo pool reuse.
pub struct MetalFrame {
    pub source: Arc<NativeFrame>,
    planes: Vec<CFRetained<CVMetalTexture>>,
    pub full_range: bool,
}
// SAFETY: these retained CoreVideo image wrappers are immutable GPU read leases;
// retain/release may occur on a queue-completion thread. The cache is not moved.
unsafe impl Send for MetalFrame {}

impl MetalFrame {
    pub fn plane_count(&self) -> usize {
        self.planes.len()
    }
    pub fn texture(
        &self,
        plane: usize,
    ) -> Option<objc2::rc::Retained<ProtocolObject<dyn objc2_metal::MTLTexture>>> {
        CVMetalTextureGetTexture(self.planes.get(plane)?)
    }
}

/// Create on the renderer's Metal device, not a separately selected default device.
/// Use on its owning renderer thread. No unsafe Send/Sync is provided for cache.
pub struct MetalFrameImporter {
    cache: CFRetained<CVMetalTextureCache>,
}

impl MetalFrameImporter {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        let mut cache = std::ptr::null_mut();
        // SAFETY: no attribute dictionaries, valid output storage, retained result.
        let result = unsafe {
            CVMetalTextureCache::create(None, None, device, None, NonNull::from(&mut cache))
        };
        if result != 0 {
            return Err(format!("Metal texture cache creation failed: {result}"));
        }
        let cache = unsafe {
            CFRetained::from_raw(NonNull::new(cache).ok_or("Metal texture cache was null")?)
        };
        Ok(Self { cache })
    }

    pub fn import(&self, source: Arc<NativeFrame>) -> Result<MetalFrame, String> {
        let format = source.pixel_format();
        let layout = if format == kCVPixelFormatType_32BGRA {
            vec![(
                MTLPixelFormat::BGRA8Unorm,
                source.width(),
                source.height(),
                0,
            )]
        } else if format == kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
            || format == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
        {
            (0..2)
                .map(|plane| {
                    (
                        if plane == 0 {
                            MTLPixelFormat::R8Unorm
                        } else {
                            MTLPixelFormat::RG8Unorm
                        },
                        CVPixelBufferGetWidthOfPlane(&source.buffer, plane),
                        CVPixelBufferGetHeightOfPlane(&source.buffer, plane),
                        plane,
                    )
                })
                .collect()
        } else {
            return Err(format!(
                "unsupported native preview pixel format: {format:#x}"
            ));
        };
        let mut planes = Vec::with_capacity(layout.len());
        for (format, width, height, plane) in layout {
            if width == 0 || height == 0 {
                return Err("empty native frame plane".into());
            }
            let mut texture = std::ptr::null_mut();
            // SAFETY: CoreVideo owns buffer storage; dimensions/plane match its
            // reported layout. Both source and returned wrappers remain retained.
            let result = unsafe {
                CVMetalTextureCache::create_texture_from_image(
                    None,
                    &self.cache,
                    &source.buffer,
                    None,
                    format,
                    width,
                    height,
                    plane,
                    NonNull::from(&mut texture),
                )
            };
            if result != 0 {
                return Err(format!("Metal frame plane import failed: {result}"));
            }
            let texture = unsafe {
                CFRetained::from_raw(NonNull::new(texture).ok_or("Metal frame plane was null")?)
            };
            if CVMetalTextureGetTexture(&texture).is_none() {
                return Err("Metal plane has no texture".into());
            }
            planes.push(texture);
        }
        Ok(MetalFrame {
            source,
            planes,
            full_range: format == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_core_video::CVPixelBufferCreate;

    fn buffer() -> CFRetained<CVPixelBuffer> {
        let mut raw = std::ptr::null_mut();
        assert_eq!(
            unsafe {
                CVPixelBufferCreate(
                    None,
                    4,
                    4,
                    kCVPixelFormatType_32BGRA,
                    None,
                    NonNull::from(&mut raw),
                )
            },
            0
        );
        unsafe { CFRetained::from_raw(NonNull::new(raw).unwrap()) }
    }

    #[test]
    fn mailbox_discards_backlog_and_leases_survive_replacement() {
        let mailbox = Arc::new(NativeFrameMailbox::default());
        let source = buffer();
        for _ in 0..100 {
            mailbox.publish(source.clone());
        }
        assert_eq!(mailbox.counters(), (100, 99, 0));
        let lease = mailbox.take_latest().unwrap();
        assert_eq!(lease.sequence, 100);
        assert_eq!((lease.width(), lease.height()), (4, 4));
        assert!(mailbox.take_latest().is_none());
        mailbox.publish(source);
        mailbox.clear();
        let weak = Arc::downgrade(&lease);
        // The retained source may move to a renderer thread and outlive clear.
        std::thread::spawn(move || {
            assert_eq!(lease.sequence, 100);
        })
        .join()
        .unwrap();
        assert!(weak.upgrade().is_none());
        assert!(mailbox.take_latest().is_none());
    }

    #[test]
    fn preview_color_rejects_missing_or_hdr_metadata() {
        use objc2_core_video::*;
        let source = buffer();
        let mailbox = NativeFrameMailbox::default();
        mailbox.publish(source.clone());
        let frame = mailbox.take_latest().unwrap();
        assert!(frame.preview_color().is_err());
        unsafe {
            source.set_attachment(
                kCVImageBufferColorPrimariesKey,
                kCVImageBufferColorPrimaries_ITU_R_709_2.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
            source.set_attachment(
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_sRGB.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
        }
        assert_eq!(frame.preview_color().unwrap(), [0; 4]);
        unsafe {
            source.set_attachment(
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_ITU_R_709_2.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
        }
        assert_eq!(frame.preview_color().unwrap(), [0, 0, 1, 0]);
        unsafe {
            source.set_attachment(
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_ITU_R_2100_HLG.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
        }
        assert!(frame.preview_color().is_err());
        unsafe {
            source.set_attachment(
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_sRGB.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
            source.set_attachment(
                kCVImageBufferColorPrimariesKey,
                kCVImageBufferColorPrimaries_P3_D65.as_ref(),
                CVAttachmentMode::ShouldPropagate,
            );
        }
        assert!(frame.preview_color().is_err());
    }
}
