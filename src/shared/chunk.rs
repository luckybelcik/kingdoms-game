use std::{collections::HashSet, sync::Arc};

use crate::shared::{
    constants::{CHUNK_POS_BITS, CHUNK_SIZE, CHUNK_VOLUME, ChunkBitRow},
    render::{chunk_draw_call_info::ChunkDrawCallInfo, vertex::Vertex},
};
use nalgebra_glm as glm;
use wgpu_buffer_allocator::allocator::{Offset, PhysicalSize, SSBOAllocator};

const DATA_PADDING_SIZE_IN_SSBO: u64 = 32;

#[derive(Clone)]
pub struct Chunk {
    chunk_pos: glm::IVec3,
    blocks: [u16; CHUNK_VOLUME],
    pub mesh: StoredChunkMesh,
    pub chunk_mask: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    pub fn get_chunk_pos(&self) -> glm::IVec3 {
        self.chunk_pos
    }

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [0; CHUNK_VOLUME],
            mesh: StoredChunkMesh {
                allocator_offset: None,
                allocated_size: None,
                chunk_draw_call_infos: Vec::new(),
            },
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            mesh: StoredChunkMesh {
                allocator_offset: None,
                allocated_size: None,
                chunk_draw_call_infos: Vec::new(),
            },
            chunk_mask: [(!0); CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    pub fn set_block(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: u16,
        dirty_chunks: &mut HashSet<glm::IVec3>,
    ) {
        if x < CHUNK_SIZE || y < CHUNK_SIZE || z < CHUNK_SIZE {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE] = block;
            if block == 0 {
                self.chunk_mask[y + z * CHUNK_SIZE] &= !(1 << x);
            } else {
                self.chunk_mask[y + z * CHUNK_SIZE] |= 1 << x;
            }

            dirty_chunks.insert(self.chunk_pos);
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn get_chunk_mask(&self) -> &[ChunkBitRow] {
        &self.chunk_mask
    }
}

pub struct SendableChunkMesh {
    pub data: Vec<u8>,
    pub lens: [usize; 6],
    pub pos: nalgebra_glm::IVec3,
}

pub type MeshJob = (Arc<Chunk>, [Option<Arc<Chunk>>; 6]);

impl SendableChunkMesh {
    pub fn make_mesh(job: &MeshJob) -> SendableChunkMesh {
        let points_right = Vec::new();
        let points_left = Vec::new();
        let points_top = Vec::new();
        let points_bottom = Vec::new();
        let points_front = Vec::new();
        let points_back = Vec::new();

        let neighbor_right = &job.1[0];
        let neighbor_left = &job.1[1];
        let neighbor_up = &job.1[2];
        let neighbor_down = &job.1[3];
        let neighbor_front = &job.1[4];
        let neighbor_back = &job.1[5];

        let right_mask = if let Some(n_right) = neighbor_right {
            n_right.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let left_mask = if let Some(n_left) = neighbor_left {
            n_left.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let top_mask = if let Some(n_top) = neighbor_up {
            n_top.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let bottom_mask = if let Some(n_bottom) = neighbor_down {
            n_bottom.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let front_mask = if let Some(n_front) = neighbor_front {
            n_front.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
        };
        let back_mask = if let Some(n_back) = neighbor_back {
            n_back.chunk_mask
        } else {
            [0; CHUNK_SIZE * CHUNK_SIZE]
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
                let current_slice = chunk.chunk_mask[i_curr];

                let xplus = (current_slice & !(current_slice << 1))
                        & !(left_mask[i_curr] >> (CHUNK_SIZE - 1));
                let xminus = (current_slice & !(current_slice >> 1))
                        & !(right_mask[i_curr] << (CHUNK_SIZE - 1));

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
        let xp_faces = swap_x_z(&xp_faces);
        let xm_faces = swap_x_z(&xm_faces);

        let mut faces = [xp_faces, xm_faces, yp_faces, ym_faces, zp_faces, zm_faces];
        let mut directions = [points_right, points_left, points_top, points_bottom, points_front, points_back];

        for i in 0..6 as usize {
            let face = &mut faces[i];
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let i_curr = y + z * CHUNK_SIZE;
                    let mut current_slice = face[i_curr];
                    let mut total_offset: usize = 0;

                    while current_slice != 0 {
                        let tz = current_slice.trailing_zeros();

                        total_offset += tz as usize; 
                        current_slice >>= tz;

                        if current_slice == 0 {
                            break;
                        }

                        let size = current_slice.trailing_ones() as usize;

                        let point: Vertex;

                        if i < 2 {
                            point = Vertex {
                                data: (z
                                    | (y << CHUNK_POS_BITS)
                                    | (total_offset << (CHUNK_POS_BITS * 2))
                                    | (size - 1 << (CHUNK_POS_BITS * 3))) as u32,
                                id: 1
                                    | ((i as u32) << 16),
                            };
                        } else {
                            point = Vertex {
                                data: (total_offset
                                    | (y << CHUNK_POS_BITS)
                                    | (z << (CHUNK_POS_BITS * 2))
                                    | (size - 1 << (CHUNK_POS_BITS * 3))) as u32,
                                id: 1
                                    | ((i as u32) << 16),
                            };
                        }

                        directions[i].push(point);

                        if size >= CHUNK_SIZE {
                            break;
                        } else {
                            current_slice >>= size;
                            total_offset += size;
                        }
                    }
                }
            }
        }

        let lens = [
            directions[0].len(), directions[1].len(), directions[2].len(), directions[3].len(),
            directions[4].len(), directions[5].len(),
        ];

        let mut points = Vec::<Vertex>::new();
        points.append(&mut directions[0]);
        points.append(&mut directions[1]);
        points.append(&mut directions[2]);
        points.append(&mut directions[3]);
        points.append(&mut directions[4]);
        points.append(&mut directions[5]);

        let data = bytemuck::cast_slice(&points).to_vec();

        SendableChunkMesh {
            data,
            lens,
            pos: chunk.chunk_pos,
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
        camera_pos: glm::Vec3,
        chunk_pos: glm::IVec3,
    ) -> Vec<ChunkDrawCallInfo> {
        let mut face_visible = [true; 6];

        let cam_chunk_pos_x = (camera_pos.x / (CHUNK_SIZE as f32)).floor() as i32;
        let cam_chunk_pos_y = (camera_pos.y / (CHUNK_SIZE as f32)).floor() as i32;
        let cam_chunk_pos_z = (camera_pos.z / (CHUNK_SIZE as f32)).floor() as i32;

        if cam_chunk_pos_x > chunk_pos.x {
            face_visible[0] = false;
        }
        if cam_chunk_pos_x < chunk_pos.x {
            face_visible[1] = false;
        }
        if cam_chunk_pos_y < chunk_pos.y {
            face_visible[2] = false;
        }
        if cam_chunk_pos_y > chunk_pos.y {
            face_visible[3] = false;
        }
        if cam_chunk_pos_z < chunk_pos.z {
            face_visible[4] = false;
        }
        if cam_chunk_pos_z > chunk_pos.z {
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

fn swap_x_z(arr: &[ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE]) -> [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE] {
    let mut new_arr = [0 as ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE];

    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let mut new_row: ChunkBitRow = 0;
            
            for z in 0..CHUNK_SIZE {
                let bit = (arr[y + z * CHUNK_SIZE] >> x) & 1;

                new_row |= bit << z;
            }
            new_arr[y + x * CHUNK_SIZE] = new_row;
        }
    }
    new_arr
}