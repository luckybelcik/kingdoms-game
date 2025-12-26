use std::sync::Arc;

use crate::{
    client::client::config::mesh_config::{MeshConfig, MeshFlags},
    shared::{
        chunk::Chunk,
        constants::{CHUNK_POS_BITS, CHUNK_SIZE, ChunkBitRow},
        coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
        render::{chunk_draw_call_info::ChunkDrawCallInfo, vertex::Vertex},
    },
};
use wgpu_buffer_allocator::allocator::{Offset, PhysicalSize, SSBOAllocator};

const DATA_PADDING_SIZE_IN_SSBO: u64 = 32;

pub struct SendableChunkMesh {
    pub data: Vec<u8>,
    pub lens: [usize; 6],
    pub pos: ChunkPos,
}

pub type MeshJob = (Arc<Chunk>, [Option<Arc<Chunk>>; 6]);

impl SendableChunkMesh {
    pub fn make_mesh(job: &MeshJob) -> SendableChunkMesh {
        let mut points_right =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());
        let mut points_left =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());
        let mut points_top =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());
        let mut points_bottom =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());
        let mut points_front =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());
        let mut points_back =
            Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<Vertex>());

        let neighbor_right = &job.1[0];
        let neighbor_left = &job.1[1];
        let neighbor_up = &job.1[2];
        let neighbor_down = &job.1[3];
        let neighbor_front = &job.1[4];
        let neighbor_back = &job.1[5];

        let right_mask = if let Some(n_right) = neighbor_right {
            &n_right.xz_swap_chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let left_mask = if let Some(n_left) = neighbor_left {
            &n_left.xz_swap_chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let top_mask = if let Some(n_top) = neighbor_up {
            &n_top.chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let bottom_mask = if let Some(n_bottom) = neighbor_down {
            &n_bottom.chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let front_mask = if let Some(n_front) = neighbor_front {
            &n_front.chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let back_mask = if let Some(n_back) = neighbor_back {
            &n_back.chunk_mask
        } else {
            &vec![0; CHUNK_SIZE * CHUNK_SIZE]
        };

        let chunk = &job.0;

        let mut xp_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];
        let mut xm_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];
        let mut yp_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];
        let mut ym_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];
        let mut zp_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];
        let mut zm_faces: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] = [0; CHUNK_SIZE * CHUNK_SIZE];

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let i_curr = y + z * CHUNK_SIZE;

                let current_swap_slice = chunk.xz_swap_chunk_mask[y + z * CHUNK_SIZE];

                let xminus_neighbor_row = if z < CHUNK_SIZE - 1 {
                    chunk.xz_swap_chunk_mask[y + (z + 1) * CHUNK_SIZE]
                } else {
                    right_mask[y + 0 * CHUNK_SIZE]
                };
                let xminus = current_swap_slice & !xminus_neighbor_row;

                let xplus_neighbor_row = if z > 0 {
                    chunk.xz_swap_chunk_mask[y + (z - 1) * CHUNK_SIZE]
                } else {
                    left_mask[y + (CHUNK_SIZE - 1) * CHUNK_SIZE]
                };
                let xplus = current_swap_slice & !xplus_neighbor_row;

                let current_slice = chunk.chunk_mask[i_curr];

                let yplus;
                if y < CHUNK_SIZE - 1 {
                    let upslice = chunk.chunk_mask[(y + 1) + z * CHUNK_SIZE];
                    yplus = current_slice & !upslice;
                } else {
                    yplus = current_slice & !top_mask[z * CHUNK_SIZE];
                }

                let yminus;
                if y != 0 {
                    let downslice = chunk.chunk_mask[(y - 1) + z * CHUNK_SIZE];
                    yminus = current_slice & !downslice;
                } else {
                    yminus = current_slice & !bottom_mask[CHUNK_SIZE - 1 + z * CHUNK_SIZE];
                }

                let zplus;
                if z < CHUNK_SIZE - 1 {
                    let front_slice = chunk.chunk_mask[y + (z + 1) * CHUNK_SIZE];
                    zplus = current_slice & !front_slice;
                } else {
                    zplus = current_slice & !front_mask[y];
                }

                let zminus;
                if z > 0 {
                    let back_slice = chunk.chunk_mask[y + (z - 1) * CHUNK_SIZE];
                    zminus = current_slice & !back_slice;
                } else {
                    zminus = current_slice & !back_mask[y + CHUNK_SIZE - 1];
                }

                xp_faces[i_curr] = xplus;
                xm_faces[i_curr] = xminus;
                yp_faces[i_curr] = yplus;
                ym_faces[i_curr] = yminus;
                zp_faces[i_curr] = zplus;
                zm_faces[i_curr] = zminus;
            }
        }

        // shadowing
        let mut yp_faces = swap_y_z(&yp_faces);
        let mut ym_faces = swap_y_z(&ym_faces);

        let faces = [
            &mut xp_faces,
            &mut xm_faces,
            &mut yp_faces,
            &mut ym_faces,
            &mut zp_faces,
            &mut zm_faces,
        ];
        let directions = [
            &mut points_right,
            &mut points_left,
            &mut points_top,
            &mut points_bottom,
            &mut points_front,
            &mut points_back,
        ];
        let mut face_amounts = [0; 6];

        for i in 0..6 as usize {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let i_curr = y + z * CHUNK_SIZE;

                    if faces[i][i_curr] == 0 {
                        continue;
                    }

                    while faces[i][i_curr] != 0 {
                        let tz = faces[i][i_curr].trailing_zeros();

                        let remaining_bits = faces[i][i_curr] >> tz;
                        let width;
                        if MeshConfig::get(MeshFlags::GREEDY_MESH) {
                            width = remaining_bits.trailing_ones() as usize;
                        } else {
                            width = 1;
                        }

                        let width_mask = if width == CHUNK_SIZE {
                            !0u32
                        } else {
                            ((1u32 << width) - 1) << tz
                        };

                        let mut height = 1;

                        if MeshConfig::get(MeshFlags::GREEDY_MESH) {
                            for h in 1..(CHUNK_SIZE - y) {
                                let next_row_idx = (y + h) + z * CHUNK_SIZE;

                                if (faces[i][next_row_idx] & width_mask) == width_mask {
                                    height += 1;

                                    faces[i][next_row_idx] &= !width_mask;
                                } else {
                                    break;
                                }
                            }
                        }

                        let point: Vertex;

                        if i < 2 {
                            point = Vertex {
                                data: (z
                                    | (y << CHUNK_POS_BITS)
                                    | ((tz as usize) << (CHUNK_POS_BITS * 2))
                                    | ((width - 1) << (CHUNK_POS_BITS * 3))
                                    | ((height - 1) << (CHUNK_POS_BITS * 4)))
                                    as u32,
                                id: 1 | ((i as u32) << 16),
                            };
                        } else if i < 4 {
                            point = Vertex {
                                data: ((tz as usize)
                                    | (z << CHUNK_POS_BITS)
                                    | (y << (CHUNK_POS_BITS * 2))
                                    | ((width - 1) << (CHUNK_POS_BITS * 3))
                                    | ((height - 1) << (CHUNK_POS_BITS * 4)))
                                    as u32,
                                id: 1 | ((i as u32) << 16),
                            };
                        } else {
                            point = Vertex {
                                data: ((tz as usize)
                                    | (y << CHUNK_POS_BITS)
                                    | (z << (CHUNK_POS_BITS * 2))
                                    | ((width - 1) << (CHUNK_POS_BITS * 3))
                                    | ((height - 1) << (CHUNK_POS_BITS * 4)))
                                    as u32,
                                id: 1 | ((i as u32) << 16),
                            };
                        }

                        directions[i].push(point);

                        face_amounts[i] += 1;
                        faces[i][i_curr] &= !width_mask;
                    }
                }
            }
        }

        let total_lens = face_amounts.iter().sum::<usize>();
        let total_byte_len = total_lens * std::mem::size_of::<Vertex>();

        let mut data = Vec::<u8>::with_capacity(total_byte_len);

        for direction_vec in directions.iter() {
            let byte_slice: &[u8] = bytemuck::cast_slice(direction_vec);
            data.extend_from_slice(byte_slice);
        }

        SendableChunkMesh {
            data,
            lens: face_amounts,
            pos: chunk.get_chunk_pos(),
        }
    }
}

#[derive(Clone)]
pub struct StoredChunkMesh {
    allocator_offset: Option<Offset>,
    allocated_size: Option<PhysicalSize>,
    chunk_draw_call_infos: Vec<ChunkDrawCallInfo>,
}

impl StoredChunkMesh {
    pub fn new_empty() -> Self {
        Self {
            allocator_offset: None,
            allocated_size: None,
            chunk_draw_call_infos: Vec::new(),
        }
    }

    pub fn get_draw_calls(&self) -> &Vec<ChunkDrawCallInfo> {
        &self.chunk_draw_call_infos
    }

    pub fn clear_draw_calls(&mut self) {
        self.chunk_draw_call_infos.clear();
    }

    pub fn update_mesh(
        &mut self,
        queue: &wgpu::Queue,
        allocator: &mut SSBOAllocator,
        sent_mesh: &SendableChunkMesh,
    ) -> bool {
        let new_offset;
        let new_size;
        if let (Some(offset), Some(size)) = (self.allocator_offset, self.allocated_size) {
            if sent_mesh.data.len() <= size as usize {
                // if data fits
                allocator
                    .modify(queue, offset, &sent_mesh.data)
                    .expect("Failed to modify chunk SSBO data");
                new_offset = offset;
                new_size = size;
            } else {
                allocator
                    .deallocate_wipe(queue, offset)
                    .expect("Failed to wipe chunk SSBO data");
                new_offset = allocator
                    .allocate(queue, &sent_mesh.data, Some(DATA_PADDING_SIZE_IN_SSBO))
                    .expect("Failed to allocate chunk on SSBO");
                new_size = sent_mesh.data.len() as u64 + DATA_PADDING_SIZE_IN_SSBO;
            }
        } else {
            new_offset = allocator
                .allocate(queue, &sent_mesh.data, Some(DATA_PADDING_SIZE_IN_SSBO))
                .expect("Failed to allocate chunk on SSBO");
            new_size = sent_mesh.data.len() as u64 + DATA_PADDING_SIZE_IN_SSBO;
        }

        let mut offset = new_offset / 4;
        let mut chunk_draw_call_infos = Vec::<ChunkDrawCallInfo>::new();
        for i in 0..sent_mesh.lens.len() {
            let current_len = sent_mesh.lens[i];
            let len_64 = current_len as u64;
            chunk_draw_call_infos.push(ChunkDrawCallInfo {
                buffer_offset: offset,
                instance_count: len_64,
                visible: true,
            });
            offset += len_64 * 2;
        }

        self.chunk_draw_call_infos = chunk_draw_call_infos;

        self.allocator_offset = Some(new_offset);
        self.allocated_size = Some(new_size);

        true
    }

    pub fn get_visible_draw_calls(
        &self,
        camera_pos: EntityPos,
        chunk_pos: ChunkPos,
    ) -> Vec<ChunkDrawCallInfo> {
        let mut face_visible = [true; 6];

        let cam_chunk_pos: ChunkPos = camera_pos.to_block_pos().into();

        if cam_chunk_pos.x > chunk_pos.x {
            face_visible[0] = false;
        }
        if cam_chunk_pos.x < chunk_pos.x {
            face_visible[1] = false;
        }
        if cam_chunk_pos.y < chunk_pos.y {
            face_visible[2] = false;
        }
        if cam_chunk_pos.y > chunk_pos.y {
            face_visible[3] = false;
        }
        if cam_chunk_pos.z < chunk_pos.z {
            face_visible[4] = false;
        }
        if cam_chunk_pos.z > chunk_pos.z {
            face_visible[5] = false;
        }

        let draw_infos_with_visibility = self
            .chunk_draw_call_infos
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let mut new_info = ChunkDrawCallInfo {
                    buffer_offset: info.buffer_offset,
                    instance_count: info.instance_count,
                    visible: info.visible,
                };
                new_info.visible = face_visible[i];
                new_info
            })
            .collect::<Vec<_>>();

        let mut chunk_draw_call_infos = Vec::<ChunkDrawCallInfo>::new();

        for info in draw_infos_with_visibility {
            if info.visible {
                chunk_draw_call_infos.push(ChunkDrawCallInfo {
                    buffer_offset: info.buffer_offset,
                    instance_count: info.instance_count,
                    visible: true,
                });
            }
        }

        chunk_draw_call_infos
    }
}

fn swap_y_z(
    arr: &[ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
) -> [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] {
    let mut new_arr = [0 as ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE];

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let original_idx = y + z * CHUNK_SIZE;
            let new_idx = z + y * CHUNK_SIZE;

            new_arr[new_idx] = arr[original_idx];
        }
    }
    new_arr
}
