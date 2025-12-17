use std::collections::HashMap;

use arc_swap::ArcSwap;

use crate::{
    client::rendering::client_chunk::ClientChunk,
    shared::coordinate_systems::{
        block_pos::BlockPos, chunk_pos::ChunkPos, chunk_relative::ChunkRelative,
        entity_pos::EntityPos,
    },
};

pub fn cast_ray_block_hit(
    camera_pos: EntityPos,
    camera_rot: nalgebra_glm::Vec3,
    chunks: &HashMap<ChunkPos, ArcSwap<ClientChunk>>,
) -> Option<(ChunkPos, ChunkRelative)> {
    let ray_pos = camera_pos;
    let mut current_block_pos: BlockPos = camera_pos.into();

    let chunk_pos: ChunkPos = current_block_pos.into();

    let chunk_relative_pos: ChunkRelative = current_block_pos.into();

    if let Some(chunk) = chunks.get(&chunk_pos)
        && chunk.load().chunk.get_block(chunk_relative_pos) != 0
    {
        return Some((chunk_pos, chunk_relative_pos));
    };

    let pitch = camera_rot.x;
    let yaw = camera_rot.y;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
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

    const MAX_DIST: i32 = 20;
    for _ in 0..MAX_DIST {
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

        let new_chunk_pos: ChunkPos = current_block_pos.into();

        let new_chunk_relative: ChunkRelative = current_block_pos.into();

        if let Some(chunk) = chunks.get(&new_chunk_pos)
            && chunk.load().chunk.get_block(new_chunk_relative) != 0
        {
            return Some((new_chunk_pos, new_chunk_relative));
        };
    }

    None
}

pub fn cast_ray_block_before(
    camera_pos: EntityPos,
    camera_rot: nalgebra_glm::Vec3,
    chunks: &HashMap<ChunkPos, ArcSwap<ClientChunk>>,
) -> Option<(ChunkPos, ChunkRelative)> {
    let ray_pos = camera_pos;
    let mut current_block_pos: BlockPos = camera_pos.into();

    let chunk_pos: ChunkPos = current_block_pos.into();

    let chunk_relative_pos: ChunkRelative = current_block_pos.into();

    if let Some(chunk) = chunks.get(&chunk_pos)
        && chunk.load().chunk.get_block(chunk_relative_pos) != 0
    {
        return Some((chunk_pos, chunk_relative_pos));
    };

    let pitch = camera_rot.x;
    let yaw = camera_rot.y;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
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

    const MAX_DIST: i32 = 64;
    for _ in 0..MAX_DIST {
        let place_pos = current_block_pos;

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

        let place_chunk_pos: ChunkPos = place_pos.into();
        let place_chunk_relative: ChunkRelative = place_pos.into();

        let current_chunk_pos: ChunkPos = current_block_pos.into();
        let current_chunk_relative: ChunkRelative = current_block_pos.into();

        if let Some(chunk) = chunks.get(&current_chunk_pos)
            && chunk.load().chunk.get_block(current_chunk_relative) != 0
        {
            return Some((place_chunk_pos, (place_chunk_relative)));
        }
    }

    None
}
