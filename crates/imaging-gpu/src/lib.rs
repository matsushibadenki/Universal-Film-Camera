//! Rust-owned SDR image renderer. Pixels never need to enter a WebView.
//!
//! Native frame import and encoder sharing can use the same pipeline later.
//! The still-image adapter deliberately reads back only the finished image.
use bytemuck::{Pod, Zeroable};
use sha2::{Digest, Sha256};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

/// Immutable display-referred sRGB cube, red-fastest, with clamped samples.
pub struct Lut3d {
    size: u32,
    rgba: Vec<[f32; 4]>,
    min: [f32; 3],
    scale: [f32; 3],
    key: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cube_validation_and_reference_cover_boundaries() {
        let samples = vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [0., 1., 1.],
            [1., 1., 1.],
        ];
        let lut = Lut3d::new(2, &samples, [0.; 3], [1.; 3]).unwrap();
        let input = [13, 47, 91, 255, 0, 0, 0, 0, 20, 32, 50, 64];
        assert_eq!(lut.reference(&input).unwrap(), input);
        assert!(lut.reference(&[1, 2, 3]).is_err());
        assert!(Lut3d::new(1, &samples, [0.; 3], [1.; 3]).is_err());
        assert!(Lut3d::new(2, &samples[..7], [0.; 3], [1.; 3]).is_err());
        assert!(Lut3d::new(2, &samples, [0.; 3], [f32::NAN; 3]).is_err());
        assert!(Lut3d::new(2, &samples, [1.; 3], [1.; 3]).is_err());
        // Metal may flush subnormal uniform values; do not silently alter domains.
        assert!(Lut3d::new(2, &samples, [-f32::MAX; 3], [f32::MAX; 3]).is_err());
        let doubled = Lut3d::new(2, &samples, [0.; 3], [2.; 3]).unwrap();
        assert_eq!(
            doubled.reference(&[200, 100, 50, 255]).unwrap(),
            [100, 50, 25, 255]
        );
    }
}

impl Lut3d {
    pub fn new(
        size: usize,
        samples: &[[f32; 3]],
        min: [f32; 3],
        max: [f32; 3],
    ) -> Result<Self, String> {
        if !(2..=65).contains(&size)
            || samples.len() != size.pow(3)
            || samples.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid LUT size or samples".into());
        }
        let scale = std::array::from_fn(|i| (1.0 / (max[i] as f64 - min[i] as f64)) as f32);
        if (0..3).any(|i| {
            !min[i].is_finite()
                || !max[i].is_finite()
                || max[i] <= min[i]
                || !scale[i].is_normal()
                || scale[i] <= 0.
        }) {
            return Err("invalid LUT domain".into());
        }
        let rgba: Vec<_> = samples
            .iter()
            .map(|c| {
                [
                    c[0].clamp(0., 1.),
                    c[1].clamp(0., 1.),
                    c[2].clamp(0., 1.),
                    1.,
                ]
            })
            .collect();
        let mut hash = Sha256::new();
        hash.update((size as u32).to_le_bytes());
        for v in min.iter().chain(max.iter()).chain(samples.iter().flatten()) {
            hash.update(v.to_le_bytes());
        }
        Ok(Self {
            size: size as u32,
            rgba,
            min,
            scale,
            key: hash.finalize().into(),
        })
    }

    /// CPU oracle used by the GPU conformance runner; premultiplied RGBA8.
    pub fn reference(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        let (pixels, remainder) = input.as_chunks::<4>();
        if !remainder.is_empty() {
            return Err("RGBA8 data is incomplete".into());
        }
        let mut output = Vec::with_capacity(input.len());
        for pixel in pixels {
            let alpha = pixel[3] as f32 / 255.;
            let p: [f32; 3] = std::array::from_fn(|i| {
                (((pixel[i] as f32 / 255. / alpha.max(1. / 255.)) - self.min[i]) * self.scale[i])
                    .clamp(0., 1.)
                    * (self.size - 1) as f32
            });
            let lo = p.map(|v| v.floor() as u32);
            let hi = lo.map(|v| (v + 1).min(self.size - 1));
            let t = p.map(|v| v.fract());
            let mut rgb = [0.; 3];
            for b in 0..2 {
                for g in 0..2 {
                    for r in 0..2 {
                        let axis = [r, g, b];
                        let index: [u32; 3] =
                            std::array::from_fn(|i| if axis[i] == 0 { lo[i] } else { hi[i] });
                        let weight = (0..3)
                            .map(|i| if axis[i] == 0 { 1. - t[i] } else { t[i] })
                            .product::<f32>();
                        let c = self.rgba
                            [((index[2] * self.size + index[1]) * self.size + index[0]) as usize];
                        for i in 0..3 {
                            rgb[i] += c[i] * weight;
                        }
                    }
                }
            }
            output.extend(rgb.map(|v| (v.clamp(0., 1.) * alpha * 255.).round() as u8));
            output.push(pixel[3]);
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RendererStats {
    pub adapter: String,
    pub backend: String,
    pub rendered_tiles: u64,
    pub lut_uploads: u64,
    pub lut_cache_hits: u64,
    pub image_allocations: u64,
    pub retained_bytes: u64,
    /// CPU wall time including upload, dispatch, GPU wait and readback; not GPU timestamps.
    pub last_render_ms: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Parameters {
    extent: [u32; 4],
    min: [f32; 4],
    scale: [f32; 4],
}

struct Images {
    width: u32,
    height: u32,
    padded_row: u32,
    input: wgpu::Texture,
    output: wgpu::Texture,
    readback: wgpu::Buffer,
}
struct Cube {
    key: [u8; 32],
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// One owner serializes dispatch and retains device, pipeline, cube and image buffers.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    cube: Option<Cube>,
    images: Option<Images>,
    binding: Option<wgpu::BindGroup>,
    stats: RendererStats,
}

impl GpuRenderer {
    pub fn new() -> Result<Self, String> {
        pollster::block_on(Self::initialize())
    }
    async fn initialize() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("GPU adapter unavailable: {e}"))?;
        let info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Universal Imaging renderer"),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("GPU device unavailable: {e}"))?;
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LUT image bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LUT pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sRGB LUT"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lut.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sRGB LUT"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = device.pop_error_scope().await {
            return Err(format!("GPU pipeline validation failed: {error}"));
        }
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LUT parameters"),
            size: std::mem::size_of::<Parameters>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            layout,
            uniform,
            cube: None,
            images: None,
            binding: None,
            stats: RendererStats {
                adapter: info.name,
                backend: format!("{:?}", info.backend),
                retained_bytes: std::mem::size_of::<Parameters>() as u64,
                ..Default::default()
            },
        })
    }

    pub fn stats(&self) -> RendererStats {
        self.stats.clone()
    }

    fn prepare(&mut self, width: u32, height: u32, lut: &Lut3d) -> Result<(), String> {
        let limits = self.device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || u64::from(width) * u64::from(height) > 2_097_152
        {
            return Err("image exceeds GPU image limits".into());
        }
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        if u64::from(row) * u64::from(height) > limits.max_buffer_size {
            return Err("image exceeds GPU readback limit".into());
        }
        if self
            .images
            .as_ref()
            .is_none_or(|i| i.width < width || i.height < height)
        {
            self.binding = None;
            // Drop old buffers before allocating the replacement resolution.
            self.images = None;
            let create = |usage| {
                self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Still working image"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage,
                    view_formats: &[],
                })
            };
            self.images = Some(Images {
                width,
                height,
                padded_row: row,
                input: create(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST),
                output: create(
                    wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                ),
                readback: self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Still encoder readback"),
                    size: u64::from(row) * u64::from(height),
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            });
            self.stats.image_allocations += 1;
        }
        if self.cube.as_ref().is_some_and(|c| c.key == lut.key) {
            self.stats.lut_cache_hits += 1;
        } else {
            self.binding = None;
            self.cube = None;
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Red-fastest float LUT"),
                size: wgpu::Extent3d {
                    width: lut.size,
                    height: lut.size,
                    depth_or_array_layers: lut.size,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                texture.as_image_copy(),
                bytemuck::cast_slice(&lut.rgba),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(lut.size * 16),
                    rows_per_image: Some(lut.size),
                },
                texture.size(),
            );
            self.cube = Some(Cube {
                key: lut.key,
                view: texture.create_view(&Default::default()),
                _texture: texture,
            });
            self.stats.lut_uploads += 1;
        }
        let images = self.images.as_ref().unwrap();
        if self.binding.is_none() {
            self.binding = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("LUT bindings"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &images.input.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &images.output.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &self.cube.as_ref().unwrap().view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            }));
        }
        let parameters = Parameters {
            extent: [width, height, lut.size, 0],
            min: [lut.min[0], lut.min[1], lut.min[2], 0.],
            scale: [lut.scale[0], lut.scale[1], lut.scale[2], 0.],
        };
        self.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&parameters));
        self.stats.retained_bytes = u64::from(images.width) * u64::from(images.height) * 8
            + u64::from(images.padded_row) * u64::from(images.height)
            + lut.rgba.len() as u64 * 16
            + std::mem::size_of::<Parameters>() as u64;
        Ok(())
    }

    pub fn render_rgba8(
        &mut self,
        width: u32,
        height: u32,
        pixels: &[u8],
        lut: &Lut3d,
    ) -> Result<Vec<u8>, String> {
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|v| v.checked_mul(4))
            .ok_or("image dimensions overflow")?;
        if expected != pixels.len() as u64 {
            return Err("RGBA8 buffer length does not match image dimensions".into());
        }
        let start = Instant::now();
        self.prepare(width, height, lut)?;
        let images = self.images.as_ref().unwrap();
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self.queue.write_texture(
            images.input.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            extent,
        );
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Still LUT dispatch"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Still LUT"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, self.binding.as_ref().unwrap(), &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        encoder.copy_texture_to_buffer(
            images.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &images.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(images.padded_row),
                    rows_per_image: Some(height),
                },
            },
            extent,
        );
        let submission = self.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        images
            .readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        let completion = self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(30)),
            })
            .map_err(|e| format!("GPU wait failed: {e}"))
            .and_then(|_| {
                rx.recv_timeout(Duration::from_secs(1))
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())
            });
        if let Err(error) = completion {
            images.readback.unmap();
            self.binding = None;
            self.images = None;
            self.stats.retained_bytes = self
                .cube
                .as_ref()
                .map_or(0, |c| u64::from(c._texture.width()).pow(3) * 16)
                + std::mem::size_of::<Parameters>() as u64;
            return Err(error);
        }
        let mapped = images.readback.slice(..).get_mapped_range();
        let mut result = Vec::with_capacity(pixels.len());
        for row in mapped
            .chunks_exact(images.padded_row as usize)
            .take(height as usize)
        {
            result.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        images.readback.unmap();
        self.stats.rendered_tiles += 1;
        self.stats.last_render_ms = start.elapsed().as_secs_f64() * 1000.;
        Ok(result)
    }

    /// Release resolution-dependent memory when leaving the capture workflow.
    pub fn release_images(&mut self) {
        self.binding = None;
        self.images = None;
        self.stats.retained_bytes = self
            .cube
            .as_ref()
            .map_or(0, |c| u64::from(c._texture.width()).pow(3) * 16)
            + std::mem::size_of::<Parameters>() as u64;
    }
}
