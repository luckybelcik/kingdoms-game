use std::collections::HashMap;

use crate::{client::rendering::{apprenderconfig::AppRenderConfig, renderer::Renderer}, shared::{chunk::Chunk, render::{push_constants::PushConstants, vertex::Vertex}}};
use wgpu::util::{DeviceExt, BufferInitDescriptor};

const QUAD_INDICES: &[u32] = &[
    0, 2, 1,
    2, 3, 1,
];


pub struct Scene {
    pub pipeline: wgpu::RenderPipeline,
    shared_quad_ibo: wgpu::Buffer,
}

impl Scene {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let pipeline = Self::create_pipeline(device, surface_format);

        let shared_quad_ibo = device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("Shared Quad IBO"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            pipeline,
            shared_quad_ibo,
        }
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut wgpu::RenderPass<'rpass>, chunks: &HashMap<nalgebra_glm::IVec3, Chunk>, camera_rot: nalgebra_glm::Vec3, camera_pos: nalgebra_glm::Vec3, aspect_ratio: f32, render_config: &AppRenderConfig) {
        renderpass.set_pipeline(&self.pipeline);
        // renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);
        PushConstants::update_render_config(renderpass, render_config);

        renderpass.set_index_buffer(self.shared_quad_ibo.slice(..), wgpu::IndexFormat::Uint32);
        
        for chunk in chunks.values().into_iter() {
            if let Some(buffer) = &chunk.mesh.get_instance_points() {
                PushConstants::update_mvp_matrix(renderpass, chunk, camera_pos, camera_rot, aspect_ratio);

                for draw_call in chunk.mesh.get_draw_calls() {
                    let start = draw_call.buffer_offset * std::mem::size_of::<Vertex>() as u64;
                    let end = start + draw_call.instance_count * std::mem::size_of::<Vertex>() as u64;
                    renderpass.set_vertex_buffer(0, buffer.slice(start..end));
                    renderpass.draw_indexed(
                        0..6,
                        0,
                        0..draw_call.instance_count as u32,
                    );
                }
            }
        }
    }

    pub fn update(
        &mut self,
        _queue: &wgpu::Queue,
    ) {
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
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstants::get_range()],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[Vertex::instance_description()],
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
}