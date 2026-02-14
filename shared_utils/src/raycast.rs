use engine_core::{
    block_pos::BlockPos, chunk_pos::ChunkPos, chunk_relative::ChunkRelative, entity_pos::EntityPos,
};
use engine_world::chunk::WorldInspector;

pub struct RaycastResult {
    pub hit: (ChunkPos, ChunkRelative),
    pub previous: (ChunkPos, ChunkRelative),
}

pub fn cast_ray<W: WorldInspector>(
    camera_pos: EntityPos,
    camera_rot: nalgebra_glm::Vec3,
    world: &W,
    max_dist: u16,
) -> Option<RaycastResult> {
    let ray_pos = camera_pos;
    let mut current_block_pos: BlockPos = camera_pos.into();

    if world.get_block_id(current_block_pos.into(), current_block_pos.into()) != 0 {
        let pos = (current_block_pos.into(), current_block_pos.into());
        return Some(RaycastResult {
            hit: pos,
            previous: pos,
        });
    }

    let (sin_pitch, cos_pitch) = camera_rot.x.sin_cos();
    let (sin_yaw, cos_yaw) = camera_rot.y.sin_cos();
    let direction =
        nalgebra_glm::vec3(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();

    const EPSILON: f32 = 1e-6;
    let mut step = nalgebra_glm::vec3(0, 0, 0);
    let mut t_delta = nalgebra_glm::vec3(f32::MAX, f32::MAX, f32::MAX);

    for i in 0..3 {
        let d = direction[i];
        if d.abs() > EPSILON {
            step[i] = if d > 0.0 { 1 } else { -1 };
            t_delta[i] = 1.0 / d.abs();
        }
    }

    let next_boundary =
        (current_block_pos.map(|c| c as f32) + step.map(|c| (c > 0) as i32 as f32)) - *ray_pos;
    let mut t_max = nalgebra_glm::vec3(f32::MAX, f32::MAX, f32::MAX);

    for i in 0..3 {
        if step[i] != 0 {
            t_max[i] = next_boundary[i].abs() * t_delta[i];
        }
    }

    for _ in 0..max_dist {
        let prev_block_pos = current_block_pos;

        if t_max.x < t_max.y {
            if t_max.x < t_max.z {
                current_block_pos.x += step.x;
                t_max.x += t_delta.x;
            } else {
                current_block_pos.z += step.z;
                t_max.z += t_delta.z;
            }
        } else if t_max.y < t_max.z {
            current_block_pos.y += step.y;
            t_max.y += t_delta.y;
        } else {
            current_block_pos.z += step.z;
            t_max.z += t_delta.z;
        }

        let chunk_p: ChunkPos = current_block_pos.into();
        let rel_p: ChunkRelative = current_block_pos.into();

        if world.get_block_id(chunk_p, rel_p) != 0 {
            return Some(RaycastResult {
                hit: (chunk_p, rel_p),
                previous: (prev_block_pos.into(), prev_block_pos.into()),
            });
        }
    }

    None
}
