//! Clip the completed wgpu frame to a rounded rectangle.
//! Only four small corner quads are touched. Multiplying destination RGBA
//! by coverage preserves premultiplied alpha and never paints a fake corner.

pub struct CornerMask {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

const SHADER: &str = r#"
struct Uniforms { size: vec2<f32>, radius: f32, padding: f32 };
@group(0) @binding(0) var<uniform> u: Uniforms;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) center: vec2<f32>,
};
@vertex
fn vertex(@builtin(vertex_index) vertex_id: u32,
          @builtin(instance_index) corner: u32) -> VertexOutput {
    let vertices = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let right = (corner & 1u) != 0u;
    let bottom = (corner & 2u) != 0u;
    let extent = u.radius + 1.0;
    let origin = vec2(select(0.0, u.size.x - extent, right),
                      select(0.0, u.size.y - extent, bottom));
    let p = origin + vertices[vertex_id] * extent;
    var out: VertexOutput;
    out.position = vec4(p.x / u.size.x * 2.0 - 1.0,
                        1.0 - p.y / u.size.y * 2.0, 0.0, 1.0);
    out.center = vec2(select(u.radius, u.size.x - u.radius, right),
                      select(u.radius, u.size.y - u.radius, bottom));
    return out;
}
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.position.xy - in.center);
    let coverage = 1.0 - smoothstep(u.radius - 0.5, u.radius + 0.5, distance);
    return vec4(0.0, 0.0, 0.0, coverage);
}
"#;

impl CornerMask {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("window-corner-uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("window-corner-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("window-corner-bind-group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("window-corner-pipeline-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("window-corner-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let multiply = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::SrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("window-corner-mask"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: multiply,
                        alpha: multiply,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            uniforms,
            bind_group,
        }
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: [f32; 2],
        radius: f32,
    ) {
        let radius = radius.min(size[0] / 2.0).min(size[1] / 2.0).max(0.0);
        if radius == 0.0 {
            return;
        }
        queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::cast_slice(&[size[0], size[1], radius, 0.0]),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("window-corner-mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..4);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires a Vulkan adapter; run explicitly for graphics validation"]
    fn gpu_mask_preserves_center_and_clears_all_four_corners() {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                force_fallback_adapter: true,
                ..Default::default()
            },
        ))
        .expect("Vulkan fallback adapter");
        let (device, queue) = futures::executor::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )
        .expect("GPU test device");
        let size = wgpu::Extent3d {
            width: 32,
            height: 24,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("corner-test-target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mask = CornerMask::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        for radius in [8.0, 0.0] {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("corner-test-readback"),
                size: 256 * 24,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.25,
                                g: 0.5,
                                b: 0.75,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
            }
            mask.render(&queue, &mut encoder, &view, [32.0, 24.0], radius);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(24),
                    },
                },
                size,
            );
            queue.submit(Some(encoder.finish()));
            let (tx, rx) = std::sync::mpsc::channel();
            buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            rx.recv().unwrap().unwrap();
            let bytes = buffer.get_mapped_range(..).unwrap();
            let pixel = |x: usize, y: usize| -> &[u8] {
                &bytes[y * 256 + x * 4..y * 256 + x * 4 + 4]
            };
            let center = pixel(16, 12);
            assert_eq!(center, &[64, 128, 191, 255]);
            for (x, y) in [(0, 0), (31, 0), (0, 23), (31, 23)] {
                if radius > 0.0 {
                    assert_eq!(pixel(x, y), &[0, 0, 0, 0], "corner {x},{y}");
                } else {
                    assert_eq!(pixel(x, y), center, "square mode {x},{y}");
                }
            }
            assert_eq!(pixel(16, 0), center, "straight top edge must remain intact");
            if radius > 0.0 {
                let edge = pixel(2, 2);
                assert!(edge[3] > 0 && edge[3] < 255, "edge must be antialiased");
                assert!(
                    (edge[0] as f32 / edge[3] as f32 - 0.25).abs() < 0.02,
                    "edge color must stay premultiplied"
                );
            }
        }
    }
}
