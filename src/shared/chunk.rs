use crate::shared::{constants::{CHUNK_SIZE, CHUNK_VOLUME}, render::{chunk_draw_call_info::ChunkDrawCallInfo, vertex::Vertex}};
use nalgebra_glm as glm;
use ssbo_allocator::allocator::{Offset, PhysicalSize, SSBOAllocator};

const DATA_PADDING_SIZE_IN_SSBO: u64 = 32;

pub struct Chunk {
    chunk_pos: glm::IVec3,
    blocks: [u16; CHUNK_VOLUME],
    pub mesh: ChunkMesh,
    pub chunk_mask: [u32; CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    pub fn get_chunk_pos(&self) -> glm::IVec3 {
        self.chunk_pos
    }

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [0; CHUNK_VOLUME],
            mesh: ChunkMesh { allocator_offset: None, allocated_size: None, is_dirty: true, chunk_draw_call_infos: Vec::new() },
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            mesh: ChunkMesh { allocator_offset: None, allocated_size: None, is_dirty: true, chunk_draw_call_infos: Vec::new() },
            chunk_mask: [0xffffffff; CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: u16) {
        if x < CHUNK_SIZE || y < CHUNK_SIZE || z < CHUNK_SIZE {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE] = block;
            if block == 0 {
                self.chunk_mask[y + z * CHUNK_SIZE] &= !(1 << x);
            } else {
                self.chunk_mask[y + z * CHUNK_SIZE] |= 1 << x;
            }
            self.mesh.is_dirty = true;
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn get_chunk_mask(&self) -> &[u32] {
        &self.chunk_mask
    }
}

pub struct ChunkMesh {
    allocator_offset: Option<Offset>,
    allocated_size: Option<PhysicalSize>,
    is_dirty: bool,
    chunk_draw_call_infos: Vec<ChunkDrawCallInfo>,
}

impl ChunkMesh {
    pub fn get_draw_calls(&self) -> &Vec<ChunkDrawCallInfo> {
        &self.chunk_draw_call_infos
    }

    pub fn clear_draw_calls(&mut self) {
        self.chunk_draw_call_infos.clear();
    }

    pub fn update_mesh(&mut self, queue: &wgpu::Queue, allocator: &mut SSBOAllocator, chunk_mask: &[u32; CHUNK_SIZE * CHUNK_SIZE], nearby_chunks: &[Option<&Chunk>; 6]) -> bool {
        if self.is_dirty == false {
            return false;
        }

        self.is_dirty = false;

        let mut points_right = Vec::new();
        let mut points_left = Vec::new();
        let mut points_top = Vec::new();
        let mut points_bottom = Vec::new();
        let mut points_front = Vec::new();
        let mut points_back = Vec::new();

        let neighbor_right = nearby_chunks[0];
        let neighbor_left  = nearby_chunks[1];
        let neighbor_up    = nearby_chunks[2];
        let neighbor_down  = nearby_chunks[3];
        let neighbor_front = nearby_chunks[4];
        let neighbor_back  = nearby_chunks[5];

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let i_curr = y + z * CHUNK_SIZE;
                let current_slice = chunk_mask[i_curr];

                let xplus;
                let xminus;
                if let Some(n_left) = neighbor_left {
                    xplus = (current_slice & !(current_slice << 1)) & !(n_left.chunk_mask[i_curr] >> 31);
                } else {
                    xplus = current_slice & !(current_slice << 1);
                }
                if let Some(n_right) = neighbor_right {
                    xminus = (current_slice & !(current_slice >> 1)) & !(n_right.chunk_mask[i_curr] << 31);
                } else {
                    xminus = current_slice & !(current_slice >> 1);
                }

                let yplus;
                if y < CHUNK_SIZE - 1 {
                    let upslice = chunk_mask[(y + 1) + z * CHUNK_SIZE];
                    yplus = current_slice & !upslice;
                } else { // chunk border top
                    if let Some(n_top) = neighbor_up {
                        let neighbor_slice = n_top.chunk_mask[0 + z * CHUNK_SIZE];
                        yplus = current_slice & !neighbor_slice;
                    } else {
                        yplus = current_slice;
                    }
                }

                let yminus;
                if y != 0 {
                    let downslice = chunk_mask[(y - 1) + z * CHUNK_SIZE];
                    yminus = current_slice & !downslice;
                } else { // chunk border bottom
                    if let Some(n_bottom) = neighbor_down {
                        let neighbor_slice = n_bottom.chunk_mask[CHUNK_SIZE - 1 + z * CHUNK_SIZE];
                        yminus = current_slice & !neighbor_slice;
                    } else {
                        yminus = current_slice;
                    }
                }

                let zplus;
                if z < CHUNK_SIZE - 1 {
                    let front_slice = chunk_mask[y + (z + 1) * CHUNK_SIZE];
                    zplus = current_slice & !front_slice;
                } else { // chunk border front
                    if let Some(n_front) = neighbor_front {
                        let neighbor_slice = n_front.chunk_mask[y + 0];
                        zplus = current_slice & !neighbor_slice;
                    } else {
                        zplus = current_slice;
                    }
                }

                let zminus;
                if z > 0 {
                    let back_slice = chunk_mask[y + (z - 1) * CHUNK_SIZE];
                    zminus = current_slice & !back_slice;
                } else { // chunk border back
                    if let Some(n_back) = neighbor_back {
                        let neighbor_slice = n_back.chunk_mask[y + CHUNK_SIZE - 1];
                        zminus = current_slice & !neighbor_slice;
                    } else {
                        zminus = current_slice;
                    }
                }

                for i in 0..32 {
                    // x plus bit
                    let xpb = xplus & (1 << i);
                    // x minus bit
                    let xmb = xminus & (1 << i);
                    let ypb = yplus & (1 << i);
                    let ymb = yminus & (1 << i);
                    let zpb = zplus & (1 << i);
                    let zmb = zminus & (1 << i);

                    //                 this zero is filler data that gets replaced VVV
                    let point = Vertex { data: (i | (y << 5) | (z << 10) | (0 << 15)) as u32, id: 1 };

                    if xpb != 0 {
                        let point = Vertex { data: point.data | ((0 as u32) << 15), id: point.id };
                        points_right.push(point)
                    }
                    if xmb != 0 {
                        let point = Vertex { data: point.data | ((1 as u32) << 15), id: point.id };
                        points_left.push(point)
                    }
                    if ypb != 0 {
                        let point = Vertex { data: point.data | ((2 as u32) << 15), id: point.id };
                        points_top.push(point)
                    }
                    if ymb != 0 {
                        let point = Vertex { data: point.data | ((3 as u32) << 15), id: point.id };
                        points_bottom.push(point)
                    }
                    if zpb != 0 {
                        let point = Vertex { data: point.data | ((4 as u32) << 15), id: point.id };
                        points_front.push(point)
                    }
                    if zmb != 0 {
                        let point = Vertex { data: point.data | ((5 as u32) << 15), id: point.id };
                        points_back.push(point)
                    }
                }
            }
        }

        let right_len = points_right.len();
        let left_len = points_left.len();
        let top_len = points_top.len();
        let bottom_len = points_bottom.len();
        let front_len = points_front.len();
        let back_len = points_back.len();

        let lens = [right_len, left_len, top_len, bottom_len, front_len, back_len];

        let mut points = Vec::<Vertex>::new();
        points.append(&mut points_right);
        points.append(&mut points_left);
        points.append(&mut points_top);
        points.append(&mut points_bottom);
        points.append(&mut points_front);
        points.append(&mut points_back);

        let data: &[u8] = bytemuck::cast_slice(&points);
        let new_offset;
        let new_size;
        if let (Some(offset), Some(size)) = (self.allocator_offset, self.allocated_size) {
            if data.len() <= size as usize { // if data fits
                allocator.modify(queue, offset, data).expect("Failed to modify chunk SSBO data");
                new_offset = offset;
                new_size = size;
            } else {
                allocator.deallocate_wipe(queue, offset).expect("Failed to wipe chunk SSBO data");
                new_offset = allocator.allocate(queue, data, Some(DATA_PADDING_SIZE_IN_SSBO)).expect("Failed to allocate chunk on SSBO");
                new_size = data.len() as u64 + DATA_PADDING_SIZE_IN_SSBO;
            }
        } else {
            new_offset = allocator.allocate(queue, data, Some(DATA_PADDING_SIZE_IN_SSBO)).expect("Failed to allocate chunk on SSBO");
            new_size = data.len() as u64 + DATA_PADDING_SIZE_IN_SSBO;
        }

        let mut offset = new_offset;
        let mut chunk_draw_call_infos = Vec::<ChunkDrawCallInfo>::new();
        for i in 0..lens.len() {
            let current_len = lens[i];
            let len_64 = current_len as u64;
            chunk_draw_call_infos.push(
                ChunkDrawCallInfo {
                    buffer_offset: offset,
                    instance_count: len_64,
                    visible: true,
            });
            offset += len_64;
        }

        self.chunk_draw_call_infos = chunk_draw_call_infos;

        self.allocator_offset = Some(new_offset);
        self.allocated_size = Some(new_size);

        true
    }

    pub fn get_visible_draw_calls(&self, camera_pos: glm::Vec3, chunk_pos: glm::IVec3) -> Vec<ChunkDrawCallInfo> {
        let mut face_visible = [true; 6];

        let cam_chunk_pos_x = ((camera_pos.x as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let cam_chunk_pos_y = ((camera_pos.y as f32) / (CHUNK_SIZE as f32)).floor() as i32;
        let cam_chunk_pos_z = ((camera_pos.z as f32) / (CHUNK_SIZE as f32)).floor() as i32;

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

        let draw_infos_with_visibility = self.chunk_draw_call_infos.iter().enumerate().map(|(i, info)| {
            let mut new_info = ChunkDrawCallInfo {
                buffer_offset: info.buffer_offset,
                instance_count: info.instance_count,
                visible: info.visible,
            };
            new_info.visible = face_visible[i];
            new_info
        }).collect::<Vec<_>>();

        let mut chunk_draw_call_infos = Vec::<ChunkDrawCallInfo>::new();

        for info in draw_infos_with_visibility {
            if info.visible {
                chunk_draw_call_infos.push(
                    ChunkDrawCallInfo {
                        buffer_offset: info.buffer_offset,
                        instance_count: info.instance_count,
                        visible: true,
                });
            }
        }

        chunk_draw_call_infos
    }
}