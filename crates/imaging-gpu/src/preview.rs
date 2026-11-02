//! SDR native preview: camera textures + LUT -> render target, with no readback.
use crate::{Cube, Lut3d};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    extent: [f32; 4],
    color: [u32; 4],
    min: [f32; 4],
    scale: [f32; 4],
    cube: [u32; 4],
}

pub struct PreviewPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    cube: Option<Cube>,
}
pub struct PreviewFrame<'a> {
    pub input: &'a [wgpu::Texture],
    pub target: &'a wgpu::TextureView,
    pub extent: [u32; 2],
    pub color: [u32; 4],
    pub lut: &'a Lut3d,
}

impl PreviewPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let texture = |binding, dimension, filterable| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable },
                view_dimension: dimension,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("native preview bindings"),
            entries: &[
                texture(0, wgpu::TextureViewDimension::D2, true),
                texture(1, wgpu::TextureViewDimension::D2, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture(3, wgpu::TextureViewDimension::D3, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native SDR preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("native LUT preview"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            layout,
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 80,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            cube: None,
        }
    }

    /// Caller owns all input textures and keeps camera leases until submission completes.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: PreviewFrame<'_>,
    ) -> Result<(), String> {
        let PreviewFrame {
            input,
            target,
            extent,
            color,
            lut,
        } = frame;
        if input.is_empty() || (color[0] != 0 && input.len() != 2) || extent.contains(&0) {
            return Err("invalid preview frame layout".into());
        }
        if self.cube.as_ref().is_none_or(|cube| cube.key != lut.key) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("preview LUT"),
                size: wgpu::Extent3d {
                    width: lut.size,
                    height: lut.size,
                    depth_or_array_layers: lut.size,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            queue.write_texture(
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
        }
        let params = Params {
            extent: [
                extent[0] as f32,
                extent[1] as f32,
                input[0].width() as f32,
                input[0].height() as f32,
            ],
            color,
            min: [lut.min[0], lut.min[1], lut.min[2], 0.0],
            scale: [lut.scale[0], lut.scale[1], lut.scale[2], 0.0],
            cube: [lut.size, 0, 0, 0],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&params));
        let y = input[0].create_view(&Default::default());
        let uv = input
            .get(1)
            .unwrap_or(&input[0])
            .create_view(&Default::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&y),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&uv),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.cube.as_ref().unwrap().view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("native LUT frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

pub struct NativeSurface {
    surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}
impl NativeSurface {
    /// # Safety
    /// `layer` must be a retained CAMetalLayer and outlive this surface and all GPU work.
    pub unsafe fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        layer: usize,
    ) -> Result<Self, String> {
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                layer as *mut _,
            ))
        }
        .map_err(|e| e.to_string())?;
        let mut config = surface
            .get_default_config(adapter, 1, 1)
            .ok_or("Metal surface unsupported")?;
        if !surface
            .get_capabilities(adapter)
            .formats
            .contains(&wgpu::TextureFormat::Bgra8Unorm)
        {
            return Err("Metal surface needs BGRA SDR output".into());
        }
        config.format = wgpu::TextureFormat::Bgra8Unorm;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        config.desired_maximum_frame_latency = 2;
        surface.configure(device, &config);
        Ok(Self { surface, config })
    }
    pub fn acquire(
        &mut self,
        device: &wgpu::Device,
        extent: [u32; 2],
    ) -> Result<wgpu::SurfaceTexture, String> {
        if extent.contains(&0) {
            return Err("native preview viewport is empty".into());
        }
        if self.config.width != extent[0] || self.config.height != extent[1] {
            self.config.width = extent[0];
            self.config.height = extent[1];
            self.surface.configure(device, &self.config);
        }
        self.surface.get_current_texture().map_err(|e| {
            self.surface.configure(device, &self.config);
            e.to_string()
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::GpuRenderer;
    fn lut(invert: bool) -> Lut3d {
        let samples = (0..8)
            .map(|i| {
                let r = (i & 1) as f32;
                [
                    if invert { 1.0 - r } else { r },
                    ((i >> 1) & 1) as f32,
                    ((i >> 2) & 1) as f32,
                ]
            })
            .collect::<Vec<_>>();
        Lut3d::new(2, &samples, [0.0; 3], [1.0; 3]).unwrap()
    }
    fn image(
        g: &GpuRenderer,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        data: &[u8],
        channels: u32,
    ) -> wgpu::Texture {
        let texture = g.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        g.queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * channels),
                rows_per_image: Some(size[1]),
            },
            texture.size(),
        );
        texture
    }
    pub(crate) fn render(
        g: &GpuRenderer,
        input: &[wgpu::Texture],
        extent: [u32; 2],
        color: [u32; 4],
        lut: &Lut3d,
    ) -> Vec<u8> {
        g.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut pipeline = PreviewPipeline::new(&g.device, wgpu::TextureFormat::Rgba8Unorm);
        let target = g.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(256 * extent[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = g.device.create_command_encoder(&Default::default());
        pipeline
            .encode(
                &g.device,
                &g.queue,
                &mut encoder,
                PreviewFrame {
                    input,
                    target: &target.create_view(&Default::default()),
                    extent,
                    color,
                    lut,
                },
            )
            .unwrap();
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(extent[1]),
                },
            },
            target.size(),
        );
        let index = g.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            tx.send(v).unwrap();
        });
        g.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        assert!(pollster::block_on(g.device.pop_error_scope()).is_none());
        let bytes = readback.slice(..).get_mapped_range();
        let result = (0..extent[1] as usize)
            .flat_map(|y| bytes[y * 256..y * 256 + extent[0] as usize * 4].to_vec())
            .collect();
        drop(bytes);
        readback.unmap();
        result
    }
    #[test]
    fn native_fragment_lut_and_aspect_fill_match_expected_pixels() {
        let g = GpuRenderer::new().unwrap();
        let bytes = [0u8, 64, 128, 255]
            .into_iter()
            .flat_map(|r| [r, 128, 192, 255])
            .collect::<Vec<_>>()
            .repeat(2);
        let input = vec![image(
            &g,
            wgpu::TextureFormat::Rgba8Unorm,
            [4, 2],
            &bytes,
            4,
        )];
        assert_eq!(render(&g, &input, [4, 2], [0; 4], &lut(false)), bytes);
        assert_eq!(
            render(&g, &input, [4, 2], [0; 4], &lut(true)),
            lut(true).reference(&bytes).unwrap()
        );
        assert_eq!(
            render(&g, &input, [2, 2], [0; 4], &lut(false)),
            [64, 128, 192, 255, 128, 128, 192, 255].repeat(2)
        );
    }
    #[test]
    fn nv12_ranges_and_rec709_transfer_render_on_gpu() {
        let g = GpuRenderer::new().unwrap();
        for (full, values) in [(0, [16, 235, 16, 235]), (1, [0, 255, 0, 255])] {
            let input = vec![
                image(&g, wgpu::TextureFormat::R8Unorm, [2, 2], &values, 1),
                image(&g, wgpu::TextureFormat::Rg8Unorm, [1, 1], &[128, 128], 2),
            ];
            let output = render(&g, &input, [2, 2], [1, full, 1, 0], &lut(false));
            for (pixel, expected) in output.chunks_exact(4).zip([0, 255, 0, 255]) {
                for value in &pixel[..3] {
                    assert!((*value as i32 - expected).abs() <= 1);
                }
            }
        }
        let input = vec![
            image(&g, wgpu::TextureFormat::R8Unorm, [2, 2], &[128; 4], 1),
            image(&g, wgpu::TextureFormat::Rg8Unorm, [1, 1], &[128, 128], 2),
        ];
        let output = render(&g, &input, [2, 2], [1, 1, 1, 0], &lut(false));
        let signal = 128.0_f64 / 255.0;
        let linear = ((signal + 0.099) / 1.099).powf(1.0 / 0.45);
        let expected = ((1.055 * linear.powf(1.0 / 2.4) - 0.055) * 255.0).round() as i32;
        assert!((output[0] as i32 - expected).abs() <= 1);
        let input = vec![
            image(&g, wgpu::TextureFormat::R8Unorm, [2, 2], &[128; 4], 1),
            image(&g, wgpu::TextureFormat::Rg8Unorm, [1, 1], &[140, 180], 2),
        ];
        for (matrix, c) in [
            (1, [1.5748, -0.187324, -0.468124, 1.8556]),
            (2, [1.402, -0.344136, -0.714136, 1.772]),
        ] {
            let output = render(&g, &input, [2, 2], [matrix, 1, 0, 0], &lut(false));
            let y = 128.0 / 255.0;
            let cb = (140.0 - 128.0) / 255.0;
            let cr = (180.0 - 128.0) / 255.0;
            for (value, expected) in
                output[..3]
                    .iter()
                    .zip([y + c[0] * cr, y + c[1] * cb + c[2] * cr, y + c[3] * cb])
            {
                assert!(
                    (*value as f64 - ((expected as f64).clamp(0.0, 1.0) * 255.0).round()).abs()
                        <= 1.0
                );
            }
        }
    }
}
