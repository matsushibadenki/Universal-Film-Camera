use imaging_gpu::{GpuRenderer, Lut3d};
use std::time::Instant;

fn table(size: usize, transform: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<[f32; 3]> {
    let mut samples = Vec::with_capacity(size.pow(3));
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                samples.push(transform([
                    r as f32 / (size - 1) as f32,
                    g as f32 / (size - 1) as f32,
                    b as f32 / (size - 1) as f32,
                ]));
            }
        }
    }
    samples
}

fn main() -> Result<(), String> {
    let start = Instant::now();
    let mut renderer = GpuRenderer::new()?;
    println!(
        "adapter={} backend={} initialization_ms={:.3}",
        renderer.stats().adapter,
        renderer.stats().backend,
        start.elapsed().as_secs_f64() * 1000.
    );
    for (label, size, width, height, min, max) in [
        ("identity-alpha-odd-width", 2, 17, 13, [0.; 3], [1.; 3]),
        ("nonlinear-swizzle", 3, 17, 13, [0.; 3], [1.; 3]),
        (
            "asymmetric-domain",
            33,
            17,
            13,
            [-0.2, 0.1, 0.],
            [1.2, 0.9, 2.],
        ),
        ("maximum-cube-and-resize", 65, 257, 129, [0.; 3], [1.; 3]),
        (
            "smaller-image-reuses-padded-buffer",
            2,
            17,
            13,
            [0.; 3],
            [1.; 3],
        ),
    ] {
        let samples = table(size, |rgb| {
            if size == 2 {
                rgb
            } else {
                [rgb[2].powi(2), rgb[0] * 1.2 - 0.1, rgb[1].sqrt()]
            }
        });
        let lut = Lut3d::new(size, &samples, min, max)?;
        let input: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                let alpha = [0u32, 64, 127, 255][i as usize % 4];
                [
                    ((i * 31 % 256) * alpha / 255) as u8,
                    ((i * 79 % 256) * alpha / 255) as u8,
                    ((i * 113 % 256) * alpha / 255) as u8,
                    alpha as u8,
                ]
            })
            .collect();
        let reference = lut.reference(&input)?;
        let before = renderer.stats();
        let gpu = renderer.render_rgba8(width, height, &input, &lut)?;
        let max_error = gpu
            .iter()
            .zip(&reference)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        if max_error > 1 {
            return Err(format!("{label}: CPU/GPU error {max_error} exceeds 1/255"));
        }
        if size == 2 && gpu != input {
            return Err("identity/alpha did not round-trip exactly".into());
        }
        let repeat = renderer.render_rgba8(width, height, &input, &lut)?;
        if repeat != gpu {
            return Err("repeat dispatch changed the result".into());
        }
        let after = renderer.stats();
        if label == "smaller-image-reuses-padded-buffer"
            && after.image_allocations != before.image_allocations
        {
            return Err("a smaller image unexpectedly reallocated GPU image buffers".into());
        }
        if after.lut_uploads != before.lut_uploads + 1
            || after.lut_cache_hits != before.lut_cache_hits + 1
        {
            return Err("LUT cache counters did not match actual reuse".into());
        }
        println!(
            "case={label} dimensions={width}x{height} cube={size} max_error_u8={max_error} warm_wall_ms={:.3} retained_bytes={}",
            after.last_render_ms, after.retained_bytes
        );
    }
    let identity = Lut3d::new(2, &table(2, |rgb| rgb), [0.; 3], [1.; 3])?;
    if renderer.render_rgba8(0, 1, &[], &identity).is_ok()
        || renderer.render_rgba8(1, 1, &[], &identity).is_ok()
        || renderer
            .render_rgba8(u32::MAX, u32::MAX, &[], &identity)
            .is_ok()
    {
        return Err("invalid image dimensions/buffer were accepted".into());
    }
    let before = renderer.stats();
    renderer.release_images();
    if renderer.stats().retained_bytes >= before.retained_bytes {
        return Err("working-image release did not reduce retained memory".into());
    }
    println!(
        "PASS rendered={} lut_uploads={} lut_cache_hits={} image_allocations={} released_retained_bytes={}",
        before.rendered_tiles,
        before.lut_uploads,
        before.lut_cache_hits,
        before.image_allocations,
        renderer.stats().retained_bytes
    );
    Ok(())
}
