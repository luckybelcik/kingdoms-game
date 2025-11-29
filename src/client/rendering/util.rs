use std::collections::HashMap;

use crate::shared::{chunk::Chunk, constants::CHUNK_SIZE};

pub fn cast_ray_block_hit(camera_pos: nalgebra_glm::Vec3, camera_rot: nalgebra_glm::Vec3, chunks: &HashMap<nalgebra_glm::IVec3, Chunk>)
    -> Option<(nalgebra_glm::IVec3, (usize, usize, usize))> {
    let ray_pos = camera_pos;
    let mut current_block_pos = nalgebra_glm::floor(&ray_pos).map(|c| c as i32);

    let (chunk_x_start, chunk_y_start, chunk_z_start) = (
        ((current_block_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32,
        ((current_block_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32,
        ((current_block_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32,
    );

    let (chunk_rel_x_start, chunk_rel_y_start, chunk_rel_z_start) = (
        wrap_to_chunk_coord(current_block_pos.x),
        wrap_to_chunk_coord(current_block_pos.y),
        wrap_to_chunk_coord(current_block_pos.z),
    );

    if let Some(chunk) = chunks.get(&nalgebra_glm::vec3(chunk_x_start, chunk_y_start, chunk_z_start)) {
        if chunk.get_block(chunk_rel_x_start, chunk_rel_y_start, chunk_rel_z_start) != 0 {
            let chunk_x = ((current_block_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32;
            let chunk_y = ((current_block_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32;
            let chunk_z = ((current_block_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32;

            let crx = wrap_to_chunk_coord(current_block_pos.x);
            let cry = wrap_to_chunk_coord(current_block_pos.y);
            let crz = wrap_to_chunk_coord(current_block_pos.z);

            let chunk_pos = nalgebra_glm::vec3(chunk_x, chunk_y, chunk_z);

            return Some((chunk_pos, (crx, cry, crz)));
        }
    };

    let pitch = camera_rot.x;
    let yaw = camera_rot.y;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let direction = nalgebra_glm::vec3(
        cos_pitch * cos_yaw,
        sin_pitch,
        cos_pitch * sin_yaw,
    ).normalize();

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

    let next_boundary = (current_block_pos.map(|c| c as f32) + step.map(|c| (c > 0) as i32 as f32)) - ray_pos;

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
        } else {
            if t_max.y < t_max.z {
                current_block_pos.y += step.y;
                t_max.y += t_delta.y;
            } else {
                current_block_pos.z += step.z;
                t_max.z += t_delta.z;
            }
        }

        let chunk_x = ((current_block_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let chunk_y = ((current_block_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let chunk_z = ((current_block_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32;

        let crx = wrap_to_chunk_coord(current_block_pos.x);
        let cry = wrap_to_chunk_coord(current_block_pos.y);
        let crz = wrap_to_chunk_coord(current_block_pos.z);

        if let Some(chunk) = chunks.get(&nalgebra_glm::vec3(chunk_x, chunk_y, chunk_z)) {
            if chunk.get_block(crx, cry, crz) != 0 {
                let chunk_pos = nalgebra_glm::vec3(chunk_x, chunk_y, chunk_z);
                
                return Some((chunk_pos, (crx, cry, crz)));
            }
        }
    }

    None
}

pub fn cast_ray_block_before(camera_pos: nalgebra_glm::Vec3, camera_rot: nalgebra_glm::Vec3, chunks: &HashMap<nalgebra_glm::IVec3, Chunk>)
    -> Option<(nalgebra_glm::IVec3, (usize, usize, usize))> {
    let ray_pos = camera_pos;
    let mut current_block_pos = nalgebra_glm::floor(&ray_pos).map(|c| c as i32);

    let (chunk_x_start, chunk_y_start, chunk_z_start) = (
        ((current_block_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32,
        ((current_block_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32,
        ((current_block_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32,
    );

    let (chunk_rel_x_start, chunk_rel_y_start, chunk_rel_z_start) = (
        wrap_to_chunk_coord(current_block_pos.x),
        wrap_to_chunk_coord(current_block_pos.y),
        wrap_to_chunk_coord(current_block_pos.z),
    );

    if let Some(chunk) = chunks.get(&nalgebra_glm::vec3(chunk_x_start, chunk_y_start, chunk_z_start)) {
        if chunk.get_block(chunk_rel_x_start, chunk_rel_y_start, chunk_rel_z_start) != 0 {
            return None;
        }
    };

    let pitch = camera_rot.x;
    let yaw = camera_rot.y;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let direction = nalgebra_glm::vec3(
        cos_pitch * cos_yaw,
        sin_pitch,
        cos_pitch * sin_yaw,
    ).normalize();

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

    let next_boundary = (current_block_pos.map(|c| c as f32) + step.map(|c| (c > 0) as i32 as f32)) - ray_pos;

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
        } else {
            if t_max.y < t_max.z {
                current_block_pos.y += step.y;
                t_max.y += t_delta.y;
            } else {
                current_block_pos.z += step.z;
                t_max.z += t_delta.z;
            }
        }

        let p_chunk_x = ((place_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let p_chunk_y = ((place_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let p_chunk_z = ((place_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32;

        let p_crx = wrap_to_chunk_coord(place_pos.x);
        let p_cry = wrap_to_chunk_coord(place_pos.y);
        let p_crz = wrap_to_chunk_coord(place_pos.z);

        let c_chunk_x = ((current_block_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let c_chunk_y = ((current_block_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let c_chunk_z = ((current_block_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32;

        let c_crx = wrap_to_chunk_coord(current_block_pos.x);
        let c_cry = wrap_to_chunk_coord(current_block_pos.y);
        let c_crz = wrap_to_chunk_coord(current_block_pos.z);

        if let Some(chunk) = chunks.get(&nalgebra_glm::vec3(c_chunk_x, c_chunk_y, c_chunk_z)) {
            if chunk.get_block(c_crx, c_cry, c_crz) != 0 {
                let chunk_pos = nalgebra_glm::vec3(p_chunk_x, p_chunk_y, p_chunk_z);
                return Some((chunk_pos, (p_crx, p_cry, p_crz)));
            }
        }
    }

    None
}

fn wrap_to_chunk_coord(world_coord: i32) -> usize {
    let result = world_coord.rem_euclid(CHUNK_SIZE as i32);
    result as usize
}