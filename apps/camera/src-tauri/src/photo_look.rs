//! Display-referred LUT baking for SDR JPEG captures. No WebView pixel transfer.
use crate::lut_catalog::LutPayload;
#[cfg(test)]
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CFType, CFURL};
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceSRGB};
#[cfg(test)]
use objc2_core_image::{CIContext, CIImage, CIVector, kCIContextWorkingColorSpace, kCIFormatRGBA8};
#[cfg(test)]
use objc2_foundation::{NSData, NSDictionary, NSNumber, NSString};
use objc2_image_io::{
    CGImageDestination, CGImageSource, kCGImageDestinationLossyCompressionQuality,
};
use std::path::Path;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

/// App-owned renderer service, shared by every window and initialized on demand.
pub struct PhotoRenderer {
    gpu: Mutex<Option<imaging_gpu::GpuRenderer>>,
    retain_images: AtomicBool,
}

impl Default for PhotoRenderer {
    fn default() -> Self {
        Self {
            gpu: Mutex::new(None),
            retain_images: AtomicBool::new(true),
        }
    }
}

impl PhotoRenderer {
    pub fn status(&self) -> Result<Option<imaging_gpu::RendererStats>, String> {
        self.gpu
            .try_lock()
            .map(|gpu| gpu.as_ref().map(|g| g.stats()))
            .map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => "renderer is busy".into(),
                std::sync::TryLockError::Poisoned(_) => "renderer lock poisoned".into(),
            })
    }

    pub fn activate(&self) {
        self.retain_images.store(true, Ordering::Release);
    }

    /// Call on a worker: wait for any in-flight still, then release its working images.
    pub fn release_images(&self) -> Result<(), String> {
        self.retain_images.store(false, Ordering::Release);
        let mut owner = self.gpu.lock().map_err(|_| "renderer lock poisoned")?;
        if let Some(gpu) = owner.as_mut() {
            gpu.release_images();
        }
        Ok(())
    }

    fn process(
        gpu: &mut imaging_gpu::GpuRenderer,
        width: u32,
        height: u32,
        pixels: &mut [u8],
        lut: &imaging_gpu::Lut3d,
    ) -> Result<(), String> {
        // LUTs are per-pixel operations, so exact tiling needs no overlap.
        // Keep GPU working images <= 2048×1024, even for 48 MP captures.
        let mut input =
            Vec::with_capacity(width.min(2048) as usize * height.min(1024) as usize * 4);
        for y in (0..height).step_by(1024) {
            for x in (0..width).step_by(2048) {
                let (tw, th) = ((width - x).min(2048), (height - y).min(1024));
                input.clear();
                for row in y..y + th {
                    let offset = ((row as usize * width as usize) + x as usize) * 4;
                    input.extend_from_slice(&pixels[offset..offset + tw as usize * 4]);
                }
                let output = gpu.render_rgba8(tw, th, &input, lut)?;
                for row in 0..th {
                    let offset = (((y + row) as usize * width as usize) + x as usize) * 4;
                    let start = row as usize * tw as usize * 4;
                    pixels[offset..offset + tw as usize * 4]
                        .copy_from_slice(&output[start..start + tw as usize * 4]);
                }
            }
        }
        Ok(())
    }

    pub fn render(
        &self,
        source: &Path,
        destination: &Path,
        lut: &LutPayload,
    ) -> Result<(), String> {
        use objc2_core_foundation::{CGPoint, CGRect, CGSize};
        use objc2_core_graphics::{
            CGBitmapContextCreate, CGBitmapContextCreateImage, CGContext, CGImage,
            CGImageAlphaInfo, CGImageByteOrderInfo,
        };
        if source == destination || destination.exists() {
            return Err("look output must be a new file".into());
        }
        let compiled =
            imaging_gpu::Lut3d::new(lut.size, &lut.samples, lut.domain_min, lut.domain_max)?;
        // Serialize decode as well as dispatch/encode across all windows. This
        // prevents queued 48 MP renders from retaining multiple full CPU images.
        let mut owner = self.gpu.lock().map_err(|_| "renderer lock poisoned")?;
        objc2::rc::autoreleasepool(|_| unsafe {
            let source_url = CFURL::from_file_path(source).ok_or("invalid photo URL")?;
            let input =
                CGImageSource::with_url(&source_url, None).ok_or("cannot decode captured JPEG")?;
            let image = input
                .image_at_index(0, None)
                .ok_or("cannot decode captured pixels")?;
            let properties = input.properties_at_index(0, None);
            let (width, height) = (CGImage::width(Some(&image)), CGImage::height(Some(&image)));
            let length = width
                .checked_mul(height)
                .and_then(|v| v.checked_mul(4))
                .ok_or("photo dimensions overflow")?;
            if width == 0
                || height == 0
                || width > 16384
                || height > 16384
                || length > 50_000_000 * 4
            {
                return Err("photo exceeds supported decode dimensions".into());
            }
            if owner.is_none() {
                *owner = Some(imaging_gpu::GpuRenderer::new()?);
            }
            let mut pixels = vec![0u8; length];
            let space =
                CGColorSpace::with_name(Some(kCGColorSpaceSRGB)).ok_or("sRGB unavailable")?;
            let flags = CGImageByteOrderInfo::Order32Big.0 | CGImageAlphaInfo::PremultipliedLast.0;
            {
                // The bitmap context borrows `pixels` only within this scope.
                let bitmap = CGBitmapContextCreate(
                    pixels.as_mut_ptr().cast(),
                    width,
                    height,
                    8,
                    width * 4,
                    Some(&space),
                    flags,
                )
                .ok_or("photo decode buffer unavailable")?;
                CGContext::draw_image(
                    Some(&bitmap),
                    CGRect::new(
                        CGPoint::new(0., 0.),
                        CGSize::new(width as f64, height as f64),
                    ),
                    Some(&image),
                );
            }
            // Do not retain a second decoded full-resolution input during dispatch.
            drop(image);
            drop(input);
            let processed = Self::process(
                owner.as_mut().unwrap(),
                width as u32,
                height as u32,
                &mut pixels,
                &compiled,
            );
            // A capture may reach the renderer after its preview was closed.
            // Such late jobs still finish, but do not re-populate the idle cache.
            if !self.retain_images.load(Ordering::Acquire) {
                owner.as_mut().unwrap().release_images();
            }
            processed?;
            // No pixel rotation: the source EXIF orientation is copied to the JPEG.
            let bitmap = CGBitmapContextCreate(
                pixels.as_mut_ptr().cast(),
                width,
                height,
                8,
                width * 4,
                Some(&space),
                flags,
            )
            .ok_or("photo encode buffer unavailable")?;
            let image = CGBitmapContextCreateImage(Some(&bitmap))
                .ok_or("photo encode image unavailable")?;
            let output_url = CFURL::from_file_path(destination).ok_or("invalid look output URL")?;
            let output = CGImageDestination::with_url(
                &output_url,
                &CFString::from_str("public.jpeg"),
                1,
                None,
            )
            .ok_or("JPEG encoder unavailable")?;
            let quality = CFDictionary::<CFType, CFType>::from_slices(
                &[kCGImageDestinationLossyCompressionQuality.as_ref()],
                &[CFNumber::new_f64(0.95).as_ref()],
            );
            output.set_properties(Some(quality.as_ref()));
            output.add_image(&image, properties.as_deref());
            if !output.finalize() {
                return Err("LUT JPEG encoding failed".into());
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, ptr::NonNull};
    #[test]
    fn renderer_stays_uninitialized_until_needed() {
        let renderer = PhotoRenderer::default();
        assert!(renderer.status().unwrap().is_none());
        renderer.release_images().unwrap();
        renderer.activate();
        assert!(renderer.status().unwrap().is_none());
    }
    #[test]
    fn tiled_render_preserves_every_pixel_and_reuses_one_working_image() {
        let lut = crate::lut_catalog::payload(Path::new("unused"), "none").unwrap();
        let compiled =
            imaging_gpu::Lut3d::new(lut.size, &lut.samples, lut.domain_min, lut.domain_max)
                .unwrap();
        let (width, height) = (2051u32, 1031u32);
        let mut pixels: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i % width % 256) as u8,
                    (i / width % 256) as u8,
                    (i.wrapping_mul(73) % 256) as u8,
                    255,
                ]
            })
            .collect();
        let expected = pixels.clone();
        let mut renderer = imaging_gpu::GpuRenderer::new().unwrap();
        PhotoRenderer::process(&mut renderer, width, height, &mut pixels, &compiled).unwrap();
        assert_eq!(pixels, expected);
        assert_eq!(renderer.stats().image_allocations, 1);
        assert_eq!(renderer.stats().lut_uploads, 1);
        assert_eq!(renderer.stats().lut_cache_hits, 3);
        assert_eq!(renderer.stats().rendered_tiles, 4);
    }
    fn render(source: &Path, destination: &Path, lut: &LutPayload) -> Result<(), String> {
        static RENDERER: std::sync::OnceLock<PhotoRenderer> = std::sync::OnceLock::new();
        RENDERER
            .get_or_init(PhotoRenderer::default)
            .render(source, destination, lut)
    }
    #[test]
    #[ignore = "opt-in end-to-end JPEG benchmark; requires a native graphics session"]
    fn benchmark_photo_look() {
        let root = std::env::temp_dir().join(format!("ufc-look-bench-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        fs::write(
            &source,
            include_bytes!("../../../../assets/design/app-icon-concept.png"),
        )
        .unwrap();
        let lut = crate::lut_catalog::payload(&root, "negative-daylight-soft").unwrap();
        let mut measurements = [Vec::new(), Vec::new()];
        for index in 0..24 {
            // Alternate ordering to reduce effects of file cache and system drift.
            for implementation in [index % 2, 1 - index % 2] {
                let output = root.join(format!("{index}-{implementation}.jpg"));
                let start = std::time::Instant::now();
                if implementation == 0 {
                    render_core_image_reference(&source, &output, &lut).unwrap();
                } else {
                    render(&source, &output, &lut).unwrap();
                }
                measurements[implementation].push(start.elapsed().as_secs_f64() * 1000.);
                fs::remove_file(output).unwrap();
            }
        }
        for (implementation, mut times) in measurements.into_iter().enumerate() {
            let cold = times.remove(0);
            times.sort_by(f64::total_cmp);
            println!(
                "photo_look implementation={} 1254x1254 cold_ms={cold:.3} warm_n={} median_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3}",
                ["core_image", "wgpu"][implementation],
                times.len(),
                times[times.len() / 2],
                times[times.len() * 95 / 100],
                times[times.len() * 99 / 100],
                times[times.len() - 1]
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
    fn pixels(path: &Path) -> (usize, usize, Vec<u8>) {
        unsafe {
            let url = CFURL::from_file_path(path).unwrap();
            let image = CGImageSource::with_url(&url, None)
                .unwrap()
                .image_at_index(0, None)
                .unwrap();
            let image = CIImage::imageWithCGImage(&image);
            let bounds = image.extent();
            let (w, h) = (bounds.size.width as usize, bounds.size.height as usize);
            let mut data = vec![0u8; w * h * 4];
            let space = CGColorSpace::with_name(Some(kCGColorSpaceSRGB)).unwrap();
            CIContext::context().render_toBitmap_rowBytes_bounds_format_colorSpace(
                &image,
                NonNull::new(data.as_mut_ptr().cast()).unwrap(),
                (w * 4) as isize,
                bounds,
                kCIFormatRGBA8,
                Some(&space),
            );
            (w, h, data)
        }
    }
    #[test]
    fn native_jpeg_bakes_cube_and_rejects_overwrite() {
        let root = std::env::temp_dir().join(format!("ufc-photo-look-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        fs::write(&source, include_bytes!("../icons/128x128@2x.png")).unwrap();
        let before = fs::read(&source).unwrap();
        let mut lut = crate::lut_catalog::payload(&root, "none").unwrap();
        let identity = root.join("identity.jpg");
        render(&source, &identity, &lut).unwrap();
        let (w, h, original) = pixels(&source);
        let (ow, oh, decoded) = pixels(&identity);
        assert_eq!((w, h), (ow, oh));
        let center = (h / 2 * w + w / 2) * 4;
        for ch in 0..3 {
            assert!((original[center + ch] as i32 - decoded[center + ch] as i32).abs() < 20);
        }
        let colored = original
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[3] == 255)
            .max_by_key(|(_, p)| p[..3].iter().max().unwrap() - p[..3].iter().min().unwrap())
            .unwrap()
            .0
            * 4;
        for ch in 0..3 {
            assert!((original[colored + ch] as i32 - decoded[colored + ch] as i32).abs() < 60);
        }
        let bw = root.join("bw.jpg");
        render(
            &source,
            &bw,
            &crate::lut_catalog::payload(&root, "bw-panchromatic-soft").unwrap(),
        )
        .unwrap();
        let (_, _, gray) = pixels(&bw);
        assert!((gray[colored] as i32 - gray[colored + 1] as i32).abs() < 10);
        assert!((gray[colored] as i32 - gray[colored + 2] as i32).abs() < 10);
        // Attach a standard EXIF orientation 6 to a real JPEG, then bake a look.
        let encoded = fs::read(&identity).unwrap();
        let mut oriented = vec![0xff, 0xd8, 0xff, 0xe1, 0, 34];
        oriented.extend_from_slice(
            b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0",
        );
        oriented.extend_from_slice(&encoded[2..]);
        let oriented_source = root.join("oriented.jpg");
        fs::write(&oriented_source, oriented).unwrap();
        let oriented_output = root.join("oriented-look.jpg");
        render(&oriented_source, &oriented_output, &lut).unwrap();
        let probe = camera_core::probe_media_resource(
            &oriented_output,
            camera_core::CapturedMediaType::Photo,
        )
        .unwrap();
        assert_eq!(probe.orientation, 6);
        assert!(probe.color.embedded_profile.is_some());
        lut.domain_max = [2.; 3];
        let half = root.join("half.jpg");
        render(&source, &half, &lut).unwrap();
        let (_, _, half_pixels) = pixels(&half);
        for ch in 0..3 {
            assert!(
                (half_pixels[center + ch] as i32 - original[center + ch] as i32 / 2).abs() < 20
            );
        }
        lut.domain_max = [1.; 3];
        lut.samples.fill([1., 0., 0.]);
        let red = root.join("red.jpg");
        render(&source, &red, &lut).unwrap();
        let (_, _, output) = pixels(&red);
        assert!(output[center] > 220 && output[center + 1] < 25 && output[center + 2] < 25);
        assert!(render(&source, &red, &lut).is_err());
        assert!(render(&source, &source, &lut).is_err());
        let service = PhotoRenderer::default();
        let identity_lut = crate::lut_catalog::payload(Path::new("unused"), "none").unwrap();
        service.release_images().unwrap();
        service
            .render(&source, &root.join("late.jpg"), &identity_lut)
            .unwrap();
        let idle_bytes = service.status().unwrap().unwrap().retained_bytes;
        service.activate();
        service
            .render(&source, &root.join("active.jpg"), &identity_lut)
            .unwrap();
        assert!(service.status().unwrap().unwrap().retained_bytes > idle_bytes);
        service.release_images().unwrap();
        assert_eq!(
            service.status().unwrap().unwrap().retained_bytes,
            idle_bytes
        );
        lut.domain_max[0] = f32::NAN;
        assert!(render(&source, &root.join("invalid.jpg"), &lut).is_err());
        assert_eq!(fs::read(&source).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
fn render_core_image_reference(
    source: &Path,
    destination: &Path,
    lut: &LutPayload,
) -> Result<(), String> {
    if source == destination || destination.exists() {
        return Err("look output must be a new file".into());
    }
    if !(2..=65).contains(&lut.size)
        || lut.samples.len() != lut.size.pow(3)
        || lut.samples.iter().flatten().any(|v| !v.is_finite())
        || (0..3).any(|i| {
            !lut.domain_min[i].is_finite()
                || !lut.domain_max[i].is_finite()
                || lut.domain_max[i] <= lut.domain_min[i]
        })
    {
        return Err("invalid LUT render payload".into());
    }
    objc2::rc::autoreleasepool(|_| unsafe {
        // Retained CF/ObjC owners remain alive throughout every filter and render call.
        let source_url = CFURL::from_file_path(source).ok_or("invalid photo URL")?;
        let input =
            CGImageSource::with_url(&source_url, None).ok_or("cannot decode captured JPEG")?;
        let pixels = input
            .image_at_index(0, None)
            .ok_or("cannot decode captured pixels")?;
        let properties = input.properties_at_index(0, None);
        // Do not rotate pixels here: ImageIO retains the original EXIF orientation.
        let image = CIImage::imageWithCGImage(&pixels);
        let space = CGColorSpace::with_name(Some(kCGColorSpaceSRGB)).ok_or("sRGB unavailable")?;
        // Core Image explicitly accepts a CGColorSpace object for this documented option.
        let space_object = &*std::ptr::from_ref(&*space).cast::<AnyObject>();
        let options = NSDictionary::from_slices(&[kCIContextWorkingColorSpace], &[space_object]);
        // Some restricted/headless environments return nil despite Apple's nonnull
        // annotation. Treat that as a recoverable render failure, not a Rust panic.
        let context: Option<objc2::rc::Retained<CIContext>> =
            objc2::msg_send![objc2::class!(CIContext), contextWithOptions: &*options];
        let context = context.ok_or("Core Image rendering context unavailable")?;
        let scale: [f64; 3] =
            std::array::from_fn(|i| 1.0 / (lut.domain_max[i] as f64 - lut.domain_min[i] as f64));
        let red = CIVector::vectorWithX_Y_Z_W(scale[0], 0., 0., 0.);
        let green = CIVector::vectorWithX_Y_Z_W(0., scale[1], 0., 0.);
        let blue = CIVector::vectorWithX_Y_Z_W(0., 0., scale[2], 0.);
        let bias = CIVector::vectorWithX_Y_Z_W(
            -lut.domain_min[0] as f64 * scale[0],
            -lut.domain_min[1] as f64 * scale[1],
            -lut.domain_min[2] as f64 * scale[2],
            0.,
        );
        let normalized = image.imageByApplyingFilter_withInputParameters(
            &NSString::from_str("CIColorMatrix"),
            &NSDictionary::from_slices(
                &[
                    &*NSString::from_str("inputRVector"),
                    &*NSString::from_str("inputGVector"),
                    &*NSString::from_str("inputBVector"),
                    &*NSString::from_str("inputBiasVector"),
                ],
                &[red.as_ref(), green.as_ref(), blue.as_ref(), bias.as_ref()],
            ),
        );
        // .cube and CIColorCube both enumerate red fastest, then green, then blue.
        let data: Vec<u8> = lut
            .samples
            .iter()
            .flat_map(|rgb| {
                [
                    rgb[0].clamp(0., 1.),
                    rgb[1].clamp(0., 1.),
                    rgb[2].clamp(0., 1.),
                    1.0,
                ]
                .into_iter()
                .flat_map(f32::to_ne_bytes)
            })
            .collect();
        let data = NSData::with_bytes(&data);
        let dimension = NSNumber::new_usize(lut.size);
        let looked = normalized.imageByApplyingFilter_withInputParameters(
            &NSString::from_str("CIColorCube"),
            &NSDictionary::from_slices(
                &[
                    &*NSString::from_str("inputCubeDimension"),
                    &*NSString::from_str("inputCubeData"),
                ],
                &[dimension.as_ref(), data.as_ref()],
            ),
        );
        let rendered = context
            .createCGImage_fromRect_format_colorSpace(
                &looked,
                image.extent(),
                kCIFormatRGBA8,
                Some(&space),
            )
            .ok_or("LUT render failed")?;
        let output_url = CFURL::from_file_path(destination).ok_or("invalid look output URL")?;
        let output =
            CGImageDestination::with_url(&output_url, &CFString::from_str("public.jpeg"), 1, None)
                .ok_or("JPEG encoder unavailable")?;
        let quality = CFDictionary::<CFType, CFType>::from_slices(
            &[kCGImageDestinationLossyCompressionQuality.as_ref()],
            &[CFNumber::new_f64(0.95).as_ref()],
        );
        output.set_properties(Some(quality.as_ref()));
        output.add_image(&rendered, properties.as_deref());
        if !output.finalize() {
            return Err("LUT JPEG encoding failed".into());
        }
        Ok(())
    })
}
