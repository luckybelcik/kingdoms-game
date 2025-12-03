use crate::{client::rendering::apprenderconfig::AppRenderConfig, shared::{chunk::Chunk, render::per_draw_data::PerDrawData}};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstants {
    pub pvm: nalgebra_glm::Mat4,
    pub render_config: AppRenderConfig,
    pub per_draw_data: PerDrawData, 
}

pub const ARG_1_SIZE: u32 = std::mem::size_of::<nalgebra_glm::Mat4>() as u32;
// the size is of a u32 here because though we store the whole apprenderconfig in pushconstants,
// we only use the first field for push constants
pub const ARG_2_SIZE: u32 = std::mem::size_of::<u32>() as u32;
pub const ARG_3_SIZE: u32 = std::mem::size_of::<PerDrawData>() as u32;
pub const PUSH_CONSTANTS_SIZE: u32 = ARG_1_SIZE + ARG_2_SIZE + ARG_3_SIZE;

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

    #[inline(always)]
    pub fn update_render_config(renderpass: &mut wgpu::RenderPass<'_>, render_config: &AppRenderConfig) {
        renderpass.set_push_constants(wgpu::ShaderStages::VERTEX, ARG_1_SIZE, bytemuck::cast_slice(&[render_config.push_constant_data]));
    }

    #[inline(always)]
    pub fn update_per_draw_data(renderpass: &mut wgpu::RenderPass<'_>, offset: u64, size: u64) {
        let data = [offset as u32, size as u32];
        renderpass.set_push_constants(wgpu::ShaderStages::VERTEX, ARG_1_SIZE + ARG_2_SIZE, bytemuck::cast_slice(&[data]));
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