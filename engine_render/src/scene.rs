use std::fs;

use bytemuck::{Pod, Zeroable};
use engine_assets::AssetManager;
use engine_core::{entity_pos::EntityPos, paths::DATA_DIR};
#[cfg(debug_assertions)]
use engine_settings::client_config::render_config::{RenderConfig, RenderFlags};
use nalgebra_glm::Vec3;
use wgpu::{
    ShaderStages,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    ChunkDrawCommand, GlobalUniformData, GlobalUniforms, push_constants::PushConstants,
    render_results::RenderResults, renderer::Renderer, texture_manager::TextureManager,
    vertex::Vertex,
};

const QUAD_INDICES: &[u32] = &[0, 2, 1, 2, 3, 1];

pub struct Scene {
    pub pipeline: wgpu::RenderPipeline,
    #[cfg(debug_assertions)]
    pub line_pipeline: wgpu::RenderPipeline,
    shared_quad_ibo: wgpu::Buffer,
    chunk_ssbo_bind_group: wgpu::BindGroup,
    pub texture_manager: TextureManager,
    global_uniforms: GlobalUniforms,
    texture_mapping_bind_group: wgpu::BindGroup,
    tables_bind_group: wgpu::BindGroup,
    pipeline_layout: wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        chunk_ssbo: &wgpu::Buffer,
        asset_manager: &AssetManager,
    ) -> Self {
        let ssbo_layout = get_chunk_ssbo_layout(device);
        let texture_manager = TextureManager::initialize(device, queue, asset_manager);
        let global_uniforms = GlobalUniforms::new(device);
        let (mapping_layout, mapping_bind_group, _) = create_storage_buffer(
            device,
            "mapping_table",
            ShaderStages::VERTEX,
            &asset_manager.texture_mapping_table,
        );
        let (tables_layout, tables_bind_group, _) = create_multi_storage_buffer(
            device,
            "tables",
            ShaderStages::FRAGMENT,
            &vec![
                bytemuck::cast_slice(&asset_manager.metadata_table),
                bytemuck::cast_slice(&asset_manager.texture_variant_mapping_table),
                bytemuck::cast_slice(&asset_manager.colormap_mask_variant_mapping_table),
            ],
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[
                &ssbo_layout,
                &texture_manager.block_array.layout,
                &global_uniforms.layout,
                &mapping_layout,
                &texture_manager.mask_array.layout,
                &texture_manager.colormap_array.layout,
                &tables_layout,
            ],
            push_constant_ranges: &[PushConstants::get_range()],
        });

        let shader_source = fs::read_to_string("shaders/shader.wgsl")
            .or_else(|_| {
                let path = DATA_DIR
                    .get()
                    .cloned()
                    .unwrap()
                    .join("native/shaders/shader.wgsl");
                fs::read_to_string(path)
            })
            .expect("Couldn't find native main shader");

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
            global_uniforms,
            texture_mapping_bind_group: mapping_bind_group,
            tables_bind_group,
            pipeline_layout,
            surface_format,
        }
    }

    pub async fn replace_shader(
        &mut self,
        device: &wgpu::Device,
        shader_source: &str,
    ) -> Result<(), String> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let info = shader_module.get_compilation_info().await;

        let errors: Vec<_> = info
            .messages
            .iter()
            .filter(|m| m.message_type == wgpu::CompilationMessageType::Error)
            .collect();

        if !errors.is_empty() {
            let mut error_log = String::from("Shader compilation failed:\n");
            for err in errors {
                error_log.push_str(&format!("  At {:?}: {}\n", err.location, err.message));
            }
            return Err(error_log);
        }

        self.pipeline = Self::build_pipeline(
            device,
            self.surface_format,
            PipelineTemplate {
                label: "Main Render Pipeline",
                poly_mode: wgpu::PolygonMode::Fill,
                shader_module: &shader_module,
                layout: &self.pipeline_layout,
            },
        );

        #[cfg(debug_assertions)]
        {
            self.line_pipeline = Self::build_pipeline(
                device,
                self.surface_format,
                PipelineTemplate {
                    label: "Debug Line Pipeline",
                    poly_mode: wgpu::PolygonMode::Line,
                    shader_module: &shader_module,
                    layout: &self.pipeline_layout,
                },
            );
        }

        Ok(())
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

        renderpass.set_bind_group(1, Some(&self.texture_manager.block_array.bind_group), &[]);

        renderpass.set_bind_group(2, &self.global_uniforms.bind_group, &[]);

        renderpass.set_bind_group(3, &self.texture_mapping_bind_group, &[]);

        renderpass.set_bind_group(4, &self.texture_manager.mask_array.bind_group, &[]);

        renderpass.set_bind_group(5, &self.texture_manager.colormap_array.bind_group, &[]);

        renderpass.set_bind_group(6, &self.tables_bind_group, &[]);

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

    pub fn update(&mut self, queue: &wgpu::Queue, time: f32) {
        let data = GlobalUniformData {
            time,
            _padding: [0.0; 3],
        };

        self.global_uniforms.update(queue, &data);
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

pub fn create_storage_buffer<T: Pod + Zeroable>(
    device: &wgpu::Device,
    name: &str,
    visibility: wgpu::ShaderStages,
    contents: &[T],
) -> (wgpu::BindGroupLayout, wgpu::BindGroup, wgpu::Buffer) {
    let empty = vec![0_u8];
    let bytes = if contents.len() == 0 {
        &empty
    } else {
        bytemuck::cast_slice(contents)
    };
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Buffer", name)),
        contents: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(&format!("{} Layout", name)),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{} Bind Group", name)),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    (layout, bind_group, buffer)
}

pub fn create_multi_storage_buffer(
    device: &wgpu::Device,
    name: &str,
    visibility: wgpu::ShaderStages,
    contents: &[&[u8]],
) -> (wgpu::BindGroupLayout, wgpu::BindGroup, Vec<wgpu::Buffer>) {
    let mut buffers = Vec::new();
    let mut layout_entries = Vec::new();

    for (i, &data) in contents.iter().enumerate() {
        let binding_idx = i as u32;
        let final_bytes = if data.is_empty() { &[0u8; 4] } else { data };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Buffer {}", name, binding_idx)),
            contents: final_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        buffers.push(buffer);

        layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: binding_idx,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }

    let mut group_entries = Vec::new();
    for (i, buffer) in buffers.iter().enumerate() {
        group_entries.push(wgpu::BindGroupEntry {
            binding: i as u32,
            resource: buffer.as_entire_binding(),
        });
    }

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(&format!("{} Layout", name)),
        entries: &layout_entries,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{} Bind Group", name)),
        layout: &layout,
        entries: &group_entries,
    });

    (layout, bind_group, buffers)
}
