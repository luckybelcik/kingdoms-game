use crate::shared::{constants::{CHUNK_SIZE, CHUNK_VOLUME}, render::{chunk_draw_call_info::ChunkDrawCallInfo, vertex::Vertex}};
use nalgebra_glm as glm;

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
            mesh: ChunkMesh { cube_mesh: None, is_dirty: true, chunk_draw_call_infos: Vec::new() },
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            mesh: ChunkMesh { cube_mesh: None, is_dirty: true, chunk_draw_call_infos: Vec::new() },
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

    pub fn generate_mesh(&mut self, device: &wgpu::Device) {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Vertex Buffer"),
                contents: bytemuck::cast_slice(&[0; 50_000]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            },
        );
        
        self.mesh.cube_mesh = Some(vertex_buffer);
    }
}

pub struct ChunkMesh {
    cube_mesh: Option<wgpu::Buffer>,
    is_dirty: bool,
    chunk_draw_call_infos: Vec<ChunkDrawCallInfo>,
}

impl ChunkMesh {
    pub fn get_instance_points(&self) -> &Option<wgpu::Buffer> {
        &self.cube_mesh
    }

    pub fn get_draw_calls(&self) -> &Vec<ChunkDrawCallInfo> {
        &self.chunk_draw_call_infos
    }

    pub fn clear_draw_calls(&mut self) {
        self.chunk_draw_call_infos.clear();
    }

    pub fn update_data(&mut self, queue: &wgpu::Queue , chunk_mask: &[u32; CHUNK_SIZE * CHUNK_SIZE], nearby_chunks: &[Option<&Chunk>; 6]) -> bool {
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
        let mut grouped_lens: Vec<Vec<usize>> = Vec::new();
        let mut previous_was_non_zero = false;
        let mut previous_vec_index = 0;

        for i in 0..6 {
            if lens[i] != 0 {
                if previous_was_non_zero {
                    grouped_lens[previous_vec_index].push(lens[i]);
                } else {
                    grouped_lens.push(vec![lens[i]]);
                }
                previous_vec_index = grouped_lens.len() - 1;
                previous_was_non_zero = true;
            } else {
                previous_was_non_zero = false;
            }
        }

        // TODO! we need to make this update every frame along with what chunk faces are visible
        let mut offset = 0;
        let mut chunk_draw_call_infos = Vec::<ChunkDrawCallInfo>::new();
        for i in 0..grouped_lens.len() {
            let current_lems = &grouped_lens[i];
            let summed_lens = (current_lems.iter().sum::<usize>()) as u64;
            chunk_draw_call_infos.push(
                ChunkDrawCallInfo {
                    buffer_offset: offset,
                    instance_count: summed_lens,
            });
            offset += summed_lens;
        }

        self.chunk_draw_call_infos = chunk_draw_call_infos;

        let mut points = Vec::<Vertex>::new();
        points.append(&mut points_right);
        points.append(&mut points_left);
        points.append(&mut points_top);
        points.append(&mut points_bottom);
        points.append(&mut points_front);
        points.append(&mut points_back);

        println!("chunk updated");

        queue.write_buffer(&self.cube_mesh.as_ref().unwrap(), 0, bytemuck::cast_slice(&points));

        true
    }
}