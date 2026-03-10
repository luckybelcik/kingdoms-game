use engine_core::chunk_pos::ChunkPos;

use crate::chunk_draw_call_info::ChunkDrawCallInfo;

pub mod block_data;
pub mod block_data_render;
pub mod chunk_draw_call_info;
pub mod constants;
pub mod gpu;
pub mod indirect;
pub mod per_draw_data;
pub mod push_constants;
pub mod render_results;
pub mod renderer;
pub mod scene;
pub mod texture_manager;
pub mod vertex;

pub struct ChunkDrawCommand {
    pub chunk_pos: ChunkPos,
    pub draw_call_info: Vec<ChunkDrawCallInfo>,
}

// align to 16 bytes or mr gpu kills us
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobalUniformData {
    pub time: f32,          // 4 bytes
    pub _padding: [f32; 3], // 12 bytes
}

pub struct GlobalUniforms {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub layout: wgpu::BindGroupLayout,
}

impl GlobalUniforms {
    pub fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Global Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    pub fn new(device: &wgpu::Device) -> Self {
        let layout = Self::create_layout(device);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Uniform Buffer"),
            size: std::mem::size_of::<GlobalUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Uniform Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            bind_group,
            layout,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: &GlobalUniformData) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}
