use std::fs;

use engine_core::entity_pos::EntityPos;
#[cfg(debug_assertions)]
use engine_settings::client_config::render_config::{RenderConfig, RenderFlags};
use image::DynamicImage;
use nalgebra_glm::Vec3;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

use crate::{
    ChunkDrawCommand, push_constants::PushConstants, render_results::RenderResults,
    renderer::Renderer, texture_manager::TextureManager, vertex::Vertex,
};

const QUAD_INDICES: &[u32] = &[0, 2, 1, 2, 3, 1];

pub struct Scene {
    pub pipeline: wgpu::RenderPipeline,
    #[cfg(debug_assertions)]
    pub line_pipeline: wgpu::RenderPipeline,
    shared_quad_ibo: wgpu::Buffer,
    chunk_ssbo_bind_group: wgpu::BindGroup,
    texture_manager: TextureManager,
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        chunk_ssbo: &wgpu::Buffer,
        atlas: &DynamicImage,
    ) -> Self {
        let ssbo_layout = get_chunk_ssbo_layout(device);
        let texture_manager = TextureManager::initialize(device, queue, atlas);
        let atlas_layout = texture_manager.get_main_atlas_bind_group_layout();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[&ssbo_layout, atlas_layout],
            push_constant_ranges: &[PushConstants::get_range()],
        });

        let shader_source = fs::read_to_string("shaders/shader.wgsl").expect("Missing shader");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline = Self::build_pipeline(
            device,
            surface_format,
            PipelineTemplate {
                label: "Main Render Pipeline",
                poly_mode: wgpu::PolygonMode::Fill,
                shader_module: &shader_module,
                layout: &pipeline_layout,
            },
        );

        #[cfg(debug_assertions)]
        let line_pipeline = Self::build_pipeline(
            device,
            surface_format,
            PipelineTemplate {
                label: "Debug Line Pipeline",
                poly_mode: wgpu::PolygonMode::Line,
                shader_module: &shader_module,
                layout: &pipeline_layout,
            },
        );

        let shared_quad_ibo = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Shared Quad IBO"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let chunk_ssbo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chunk SSBO Bind Group"),
            layout: &ssbo_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_ssbo.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            #[cfg(debug_assertions)]
            line_pipeline,
            shared_quad_ibo,
            chunk_ssbo_bind_group,
            texture_manager,
        }
    }
}

struct PipelineTemplate<'a> {
    label: &'a str,
    poly_mode: wgpu::PolygonMode,
    shader_module: &'a wgpu::ShaderModule,
    layout: &'a wgpu::PipelineLayout,
}

impl Scene {
    fn build_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        template: PipelineTemplate,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(template.label),
            layout: Some(template.layout),
            vertex: wgpu::VertexState {
                module: template.shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[], // SSBO driven
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: template.poly_mode,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Renderer::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: template.shader_module,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }

    pub fn render<'rpass>(
        &'rpass self,
        renderpass: &mut wgpu::RenderPass<'rpass>,
        camera_pos: &EntityPos,
        camera_rot: &Vec3,
        draw_commands: &Vec<ChunkDrawCommand>,
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

        renderpass.set_bind_group(
            1,
            Some(self.texture_manager.get_main_atlas_bind_group()),
            &[],
        );

        PushConstants::update_render_config(renderpass);

        renderpass.set_index_buffer(self.shared_quad_ibo.slice(..), wgpu::IndexFormat::Uint32);

        let mut results = RenderResults::default();
        PushConstants::update_vp_matrix(renderpass, *camera_pos, *camera_rot, aspect_ratio);

        for draw_command in draw_commands {
            if draw_command.draw_call_info.is_empty() {
                continue;
            }

            PushConstants::update_chunk_pos(renderpass, draw_command.chunk_pos);

            for draw_call in draw_command.draw_call_info.iter() {
                let start = draw_call.buffer_offset * std::mem::size_of::<Vertex>() as u64;
                let end = start + draw_call.instance_count * std::mem::size_of::<Vertex>() as u64;

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

        results
    }

    pub fn update(&mut self, _queue: &wgpu::Queue) {
        // this method will be used later for the SSBO
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
