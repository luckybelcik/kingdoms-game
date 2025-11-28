use crate::{client::rendering::renderer::Renderer, shared::{chunk::Chunk, render::{indirect::create_mdi_commands, push_constants::PushConstants, vertex::Vertex}}};
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

    pub fn render<'rpass>(&'rpass self, renderpass: &mut wgpu::RenderPass<'rpass>, chunks: &Vec<Chunk>, camera_rot: nalgebra_glm::Vec3, camera_pos: nalgebra_glm::Vec3, aspect_ratio: f32) {
        renderpass.set_pipeline(&self.pipeline);
        // renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);
        PushConstants::update_view_projection_matrix(renderpass, camera_pos, camera_rot, aspect_ratio);

        renderpass.set_index_buffer(self.shared_quad_ibo.slice(..), wgpu::IndexFormat::Uint32);
        
        for chunk in chunks.iter() {
            if let Some(mesh) = &chunk.mesh {
                let full_buffer = mesh.get_instance_points();
                
                renderpass.set_vertex_buffer(0, mesh.get_instance_points().slice(..));
            
                PushConstants::update_model_matrix(renderpass, chunk);
                //renderpass.multi_draw_indexed_indirect(mesh.get_indirect_buffer(), 0, 6);

                
                // fallback:
                if let Some(infos) = &chunk.infos {
                    let mdi_commands = create_mdi_commands(infos, camera_rot);

                    let mut i = 0;
                    for info in infos.iter() {
                        // if no faces then draw next
                        if mdi_commands[i].instance_count == 0 {
                            continue;
                        }

                        let start_byte = (info.offset as wgpu::BufferAddress) * std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
                        let end_byte = start_byte + (info.count as wgpu::BufferAddress) * std::mem::size_of::<Vertex>() as wgpu::BufferAddress;

                        renderpass.set_vertex_buffer(0, full_buffer.slice(start_byte..end_byte));
                        let index_count = 6;
                        renderpass.draw_indexed(
                            0..index_count,
                            0,
                            0..info.count,
                        );

                        i += 1;
                    }
                }
                
            }
        }
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
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