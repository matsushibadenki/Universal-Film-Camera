//! Zero-copy CoreVideo → Metal → wgpu input. Presentation is a separate stage.
use camera_apple::native_frame::{MetalFrame, MetalFrameImporter, NativeFrame};
use foreign_types::{ForeignType, ForeignTypeRef};
use objc2::runtime::ProtocolObject;
use std::sync::Arc;

pub struct NativeTextureImporter {
    device: wgpu::Device,
    metal: MetalFrameImporter,
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFType};
    use objc2_core_video::*;
    use std::{ptr::NonNull, sync::mpsc, time::Duration};

    fn buffer(format: u32) -> CFRetained<CVPixelBuffer> {
        let empty = CFDictionary::<CFType, CFType>::empty();
        let attributes = CFDictionary::<CFType, CFType>::from_slices(
            &[
                unsafe { kCVPixelBufferIOSurfacePropertiesKey }.as_ref(),
                unsafe { kCVPixelBufferMetalCompatibilityKey }.as_ref(),
            ],
            &[empty.as_ref(), CFBoolean::new(true).as_ref()],
        );
        let mut raw = std::ptr::null_mut();
        assert_eq!(
            unsafe {
                CVPixelBufferCreate(
                    None,
                    4,
                    4,
                    format,
                    Some(attributes.as_opaque()),
                    NonNull::from(&mut raw),
                )
            },
            0
        );
        let buffer = unsafe { CFRetained::from_raw(NonNull::new(raw).unwrap()) };
        let flags = CVPixelBufferLockFlags::empty();
        assert_eq!(unsafe { CVPixelBufferLockBaseAddress(&buffer, flags) }, 0);
        if format == kCVPixelFormatType_32BGRA {
            let base = CVPixelBufferGetBaseAddress(&buffer).cast::<u8>();
            for y in 0..4 {
                for x in 0..4 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            [13u8, 47, 91, 255].as_ptr(),
                            base.add(y * CVPixelBufferGetBytesPerRow(&buffer) + x * 4),
                            4,
                        )
                    };
                }
            }
        } else {
            for plane in 0..2 {
                let base = CVPixelBufferGetBaseAddressOfPlane(&buffer, plane).cast::<u8>();
                unsafe {
                    std::ptr::write_bytes(
                        base,
                        128,
                        CVPixelBufferGetBytesPerRowOfPlane(&buffer, plane)
                            * CVPixelBufferGetHeightOfPlane(&buffer, plane),
                    )
                };
            }
        }
        assert_eq!(unsafe { CVPixelBufferUnlockBaseAddress(&buffer, flags) }, 0);
        buffer
    }

    #[test]
    fn corevideo_metal_wgpu_import_preserves_bgra_and_nv12_planes() {
        let renderer = crate::GpuRenderer::new().unwrap();
        let importer = NativeTextureImporter::new(&renderer.device).unwrap();
        let mailbox = camera_apple::native_frame::NativeFrameMailbox::default();
        mailbox.publish(buffer(kCVPixelFormatType_32BGRA));
        let frame = mailbox.take_latest().unwrap();
        let weak = Arc::downgrade(&frame);
        // SAFETY: the imported lease remains alive through the blocking GPU wait.
        let imported = unsafe { importer.import(frame) }.unwrap();
        assert_eq!(imported.planes[0].format(), wgpu::TextureFormat::Bgra8Unorm);
        assert!(!imported.full_range);
        let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            imported.planes[0].as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        let submission = renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(
                    &bytes[y * 256 + x * 4..y * 256 + x * 4 + 4],
                    &[13, 47, 91, 255]
                );
            }
        }
        drop(bytes);
        readback.unmap();
        let samples = (0..8)
            .map(|i| {
                [
                    1.0 - (i & 1) as f32,
                    ((i >> 1) & 1) as f32,
                    ((i >> 2) & 1) as f32,
                ]
            })
            .collect::<Vec<_>>();
        let lut = crate::Lut3d::new(2, &samples, [0.0; 3], [1.0; 3]).unwrap();
        let rendered =
            crate::preview::tests::render(&renderer, &imported.planes, [4, 4], [0; 4], &lut);
        for pixel in rendered.chunks_exact(4) {
            assert_eq!(pixel, &[164, 47, 13, 255]);
        }
        drop(imported);
        assert!(weak.upgrade().is_none());
        for format in [
            kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
            kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        ] {
            mailbox.publish(buffer(format));
            let imported = unsafe { importer.import(mailbox.take_latest().unwrap()) }.unwrap();
            assert_eq!(imported.planes.len(), 2);
            assert_eq!(imported.planes[0].format(), wgpu::TextureFormat::R8Unorm);
            assert_eq!(imported.planes[1].format(), wgpu::TextureFormat::Rg8Unorm);
            assert_eq!(imported.planes[1].width(), 2);
            assert_eq!(
                imported.full_range,
                format == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
            );
        }
    }
}

/// Keep this lease alive until the submitted GPU work has completed, not merely
/// until encoding or queue.submit returns. Never send it to a WebView.
pub struct ImportedNativeFrame {
    _lease: MetalFrame,
    pub planes: Vec<wgpu::Texture>,
    pub full_range: bool,
    pub sequence: u64,
}

impl NativeTextureImporter {
    pub fn new(device: &wgpu::Device) -> Result<Self, String> {
        // SAFETY: borrow only; do not destroy or mutate the HAL device.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }
            .ok_or("native camera input requires a Metal wgpu device")?;
        let raw = hal.raw_device().lock();
        // Both bindings describe the same Objective-C id<MTLDevice>. The HAL
        // guard and mutex retain the device for this borrowed reference.
        let objc_device =
            unsafe { &*(raw.as_ptr() as *const ProtocolObject<dyn objc2_metal::MTLDevice>) };
        let metal = MetalFrameImporter::new(objc_device)?;
        Ok(Self {
            device: device.clone(),
            metal,
        })
    }

    /// # Safety
    /// The returned lease must outlive ALL GPU work using its textures/views.
    /// Do not submit work using cloned views after dropping the lease. Use the
    /// same device/queue and keep planes read-only; no writes to camera buffers.
    pub unsafe fn import(&self, frame: Arc<NativeFrame>) -> Result<ImportedNativeFrame, String> {
        let lease = self.metal.import(frame)?;
        let sequence = lease.source.sequence;
        let full_range = lease.full_range;
        let mut planes = Vec::with_capacity(lease.plane_count());
        for plane in 0..lease.plane_count() {
            let native = lease.texture(plane).ok_or("missing native texture plane")?;
            let raw = unsafe {
                metal::TextureRef::from_ptr(objc2::rc::Retained::as_ptr(&native) as *mut _)
            }
            .to_owned();
            let format = match raw.pixel_format() {
                metal::MTLPixelFormat::BGRA8Unorm => wgpu::TextureFormat::Bgra8Unorm,
                metal::MTLPixelFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
                metal::MTLPixelFormat::RG8Unorm => wgpu::TextureFormat::Rg8Unorm,
                _ => return Err("unexpected imported Metal plane format".into()),
            };
            let size = wgpu::Extent3d {
                width: raw.width() as u32,
                height: raw.height() as u32,
                depth_or_array_layers: 1,
            };
            // SAFETY: the CoreVideo cache uses this exact wgpu device. Existing
            // camera pixels are initialized; format and extent come from Metal.
            let hal_texture = unsafe {
                wgpu::hal::metal::Device::texture_from_raw(
                    raw,
                    format,
                    metal::MTLTextureType::D2,
                    1,
                    1,
                    size.into(),
                )
            };
            let texture = unsafe {
                self.device
                    .create_texture_from_hal::<wgpu::hal::api::Metal>(
                        hal_texture,
                        &wgpu::TextureDescriptor {
                            label: Some("native camera input plane"),
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::COPY_SRC,
                            view_formats: &[],
                        },
                    )
            };
            planes.push(texture);
        }
        Ok(ImportedNativeFrame {
            _lease: lease,
            planes,
            full_range,
            sequence,
        })
    }
}
