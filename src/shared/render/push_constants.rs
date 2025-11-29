use crate::shared::{chunk::Chunk};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstants {
    pub pvm: nalgebra_glm::Mat4,
}

pub const PUSH_CONSTANTS_SIZE: u32 = std::mem::size_of::<PushConstants>() as u32;

impl PushConstants {
    pub fn get_range() -> wgpu::PushConstantRange {
        wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX,
            range: 0..PUSH_CONSTANTS_SIZE,
        }
    }

    #[inline(always)]
    pub fn update_mvp_matrix(renderpass: &mut wgpu::RenderPass<'_>, chunk: &Chunk, camera_pos: nalgebra_glm::Vec3, camera_rot: nalgebra_glm::Vec3, aspect_ratio: f32) {
        let model_matrix = nalgebra_glm::translate(&nalgebra_glm::Mat4::identity(), &(chunk.get_chunk_pos().map(|x| x as f32) * 32 as f32));
        let projection = nalgebra_glm::perspective_lh_zo(aspect_ratio, 80_f32.to_radians(), 0.1, 1000.0);
        let view = nalgebra_glm::look_at_lh(&camera_pos, &(camera_pos + camera_forward(camera_rot)), &nalgebra_glm::Vec3::y());
        renderpass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::cast_slice(&[projection * view * model_matrix]));
    }
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