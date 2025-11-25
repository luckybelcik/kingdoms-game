use crate::{client::rendering::{renderer::Renderer}, shared::{chunk::Chunk, constants::CHUNK_SIZE, render::{Vertex, create_mdi_commands}}};
use wgpu::util::{DeviceExt, BufferInitDescriptor};

const QUAD_VERTICES: &[f32] = &[
    0.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    0.0, 1.0, 0.0,
    1.0, 1.0, 0.0,
];

const QUAD_INDICES: &[u32] = &[
    0, 2, 1,
    2, 3, 1,
];


pub struct Scene {
    pub uniform: UniformBinding,
    pub pipeline: wgpu::RenderPipeline,
    shared_quad_vbo: wgpu::Buffer,
    shared_quad_ibo: wgpu::Buffer,
}

impl Scene {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let uniform = UniformBinding::new(device);
        let pipeline = Self::create_pipeline(device, surface_format, &uniform);

        let shared_quad_vbo = device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("Shared Quad VBO"),
                contents: bytemuck::cast_slice(QUAD_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let shared_quad_ibo = device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("Shared Quad IBO"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            uniform,
            pipeline,
            shared_quad_vbo,
            shared_quad_ibo,
        }
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut wgpu::RenderPass<'rpass>, chunks: &Vec<Chunk>, camera_rot: nalgebra_glm::Vec3) {
        renderpass.set_pipeline(&self.pipeline);
        renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);

        renderpass.set_vertex_buffer(0, self.shared_quad_vbo.slice(..));
        renderpass.set_index_buffer(self.shared_quad_ibo.slice(..), wgpu::IndexFormat::Uint32);
        
        for chunk in chunks.iter() {
            if let Some(mesh) = &chunk.mesh {
                let full_buffer = mesh.get_instance_points();
                
                renderpass.set_vertex_buffer(1, mesh.get_instance_points().slice(..));
                
                let model_matrix = nalgebra_glm::translate(&nalgebra_glm::Mat4::identity(), &(chunk.get_chunk_pos().map(|x| x as f32) * CHUNK_SIZE as f32));
                renderpass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::cast_slice(&[model_matrix]));
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

                        renderpass.set_vertex_buffer(0, self.shared_quad_vbo.slice(..));
                        renderpass.set_vertex_buffer(1, full_buffer.slice(start_byte..end_byte));
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
        aspect_ratio: f32,
        camera_pos: nalgebra_glm::Vec3,
        camera_rot: nalgebra_glm::Vec3,
    ) {
        let projection =
            nalgebra_glm::perspective_lh_zo(aspect_ratio, 80_f32.to_radians(), 0.1, 1000.0);
        let view = nalgebra_glm::look_at_lh(&camera_pos, &(camera_pos + Self::camera_forward(camera_rot)), &nalgebra_glm::Vec3::y());

        self.uniform.update_buffer(
            queue,
            0,
            UniformBuffer {
                vp: projection * view,
            },
        );
    }

    fn camera_forward(camera_rot: nalgebra_glm::Vec3) -> nalgebra_glm::Vec3 {
        let (sin_pitch, cos_pitch) = camera_rot.x.sin_cos();
        let (sin_yaw, cos_yaw) = camera_rot.y.sin_cos();
        nalgebra_glm::vec3(
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw,
        ).normalize()
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        uniform: &UniformBinding,
    ) -> wgpu::RenderPipeline {
        let shader_source = include_str!("../shaders/shader.wgsl");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform.bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<nalgebra_glm::Mat4>() as u32,
            }],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[Vertex::vertex_description(), Vertex::instance_description()],
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

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformBuffer {
    vp: nalgebra_glm::Mat4,
}

pub struct UniformBinding {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl UniformBinding {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[UniformBuffer::default()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update_buffer(
        &mut self,
        queue: &wgpu::Queue,
        offset: wgpu::BufferAddress,
        uniform_buffer: UniformBuffer,
    ) {
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[uniform_buffer]),
        )
    }
}