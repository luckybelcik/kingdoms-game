use crate::client::client::config::render_config::{RenderConfig, RenderFlags};
use crate::client::rendering::gpu::get_texture_bind_group_layout;
use crate::{
    client::{
        client::client::Client,
        rendering::{render_results::RenderResults, renderer::Renderer},
    },
    shared::render::{push_constants::PushConstants, vertex::Vertex},
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

const QUAD_INDICES: &[u32] = &[0, 2, 1, 2, 3, 1];

pub struct Scene {
    pub pipeline: wgpu::RenderPipeline,
    #[cfg(debug_assertions)]
    pub line_pipeline: wgpu::RenderPipeline,
    shared_quad_ibo: wgpu::Buffer,
    chunk_ssbo_bind_group: wgpu::BindGroup,
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        chunk_ssbo: &wgpu::Buffer,
    ) -> Self {
        let pipeline = Self::create_pipeline(device, surface_format);

        let shared_quad_ibo = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Shared Quad IBO"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let chunk_ssbo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chunk SSBO Bind Group"),
            layout: &get_chunk_ssbo_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_ssbo.as_entire_binding(),
            }],
        });

        #[cfg(not(debug_assertions))]
        return Self {
            pipeline,
            shared_quad_ibo,
            chunk_ssbo_bind_group,
        };

        #[cfg(debug_assertions)]
        Self {
            pipeline,
            line_pipeline: Self::create_line_pipeline(device, surface_format),
            shared_quad_ibo,
            chunk_ssbo_bind_group,
        }
    }

    pub fn render<'rpass>(
        &'rpass self,
        renderpass: &mut wgpu::RenderPass<'rpass>,
        client: &mut Option<Client>,
        aspect_ratio: f32,
    ) -> RenderResults {
        #[cfg(debug_assertions)]
        if RenderConfig::get(RenderFlags::LINE_RENDERING) {
            renderpass.set_pipeline(&self.line_pipeline);
        } else {
            renderpass.set_pipeline(&self.pipeline);
        }

        #[cfg(not(debug_assertions))]
        renderpass.set_pipeline(&self.pipeline);

        renderpass.set_bind_group(0, &self.chunk_ssbo_bind_group, &[]);
        PushConstants::update_render_config(renderpass);

        renderpass.set_index_buffer(self.shared_quad_ibo.slice(..), wgpu::IndexFormat::Uint32);

        let mut results = RenderResults::default();

        if let Some(client) = client {
            PushConstants::update_vp_matrix(
                renderpass,
                client.camera_pos,
                client.camera_rot,
                aspect_ratio,
            );

            for client_chunk in client.chunks.values() {
                let chunk = client_chunk.chunk.load();
                let mesh = client_chunk.mesh.load();

                let mut draw_calls = mesh.get_draw_calls();
                let culled_calls =
                    mesh.get_visible_draw_calls(client.camera_pos, chunk.get_chunk_pos());

                if culled_calls.is_empty() {
                    continue;
                }

                PushConstants::update_chunk_pos(renderpass, chunk.get_chunk_pos());

                if RenderConfig::get(RenderFlags::CULL_FACES) {
                    draw_calls = &culled_calls;
                }

                for draw_call in draw_calls {
                    let start = draw_call.buffer_offset * std::mem::size_of::<Vertex>() as u64;
                    let end =
                        start + draw_call.instance_count * std::mem::size_of::<Vertex>() as u64;

                    PushConstants::update_per_draw_data(
                        renderpass,
                        draw_call.buffer_offset,
                        draw_call.instance_count,
                    );

                    if start >= end {
                        // next iteration if nothing to draw
                        continue;
                    }

                    renderpass.draw_indexed(0..6, 0, 0..draw_call.instance_count as u32);

                    results.triangles_rendered += draw_call.instance_count as u32 * 2;
                    results.draw_calls += 1;
                }
                results.chunk_count += 1;
            }
        }

        results
    }

    pub fn update(&mut self, _queue: &wgpu::Queue) {
        // this method will be used later for the SSBO
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader_source = include_str!("../shaders/shader.wgsl");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &get_chunk_ssbo_layout(device),
                &get_texture_bind_group_layout(device),
            ],
            push_constant_ranges: &[PushConstants::get_range()],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Renderer::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg(debug_assertions)]
    fn create_line_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader_source = include_str!("../shaders/shader.wgsl");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &get_chunk_ssbo_layout(device),
                &get_texture_bind_group_layout(device),
            ],
            push_constant_ranges: &[PushConstants::get_range()],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Line,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Renderer::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }
}

fn get_chunk_ssbo_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Storage Buffer Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}
