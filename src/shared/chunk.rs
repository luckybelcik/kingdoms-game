use crate::shared::{constants::{CHUNK_SIZE, CHUNK_VOLUME}, render::{indirect::NormalGroupInfo, vertex::Vertex}};
use nalgebra_glm as glm;

pub struct Chunk {
    chunk_pos: glm::IVec3,
    blocks: [u16; CHUNK_VOLUME],
    pub mesh: Option<ChunkMesh>,
    pub infos: Option<[NormalGroupInfo; 6]>,
    is_dirty: bool,
    chunk_mask: [u32; CHUNK_SIZE * CHUNK_SIZE]
}

impl Chunk {
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn get_chunk_pos(&self) -> glm::IVec3 {
        self.chunk_pos
    }

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [0; CHUNK_VOLUME],
            mesh: None,
            infos: None,
            is_dirty: true,
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            mesh: None,
            infos: None,
            is_dirty: true,
            chunk_mask: [0xffffffff; CHUNK_SIZE * CHUNK_SIZE]
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

    pub fn generate_mesh(&mut self, device: &wgpu::Device) -> bool {
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

        let normals: [[i8; 3]; 6] = [
            // Right (+X)
            [1, 0, 0],
            // Left (-X)
            [-1, 0, 0],
            // Top (+Y)
            [0, 1, 0],
            // Bottom (-Y)
            [0, -1, 0],
            // Front (+Z)
            [0, 0, 1],
            // Back (-Z)
            [0, 0, -1],
        ];

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let current_slice = self.chunk_mask[y + z * CHUNK_SIZE];

                // Faces along +X and -X are found by shifting the bits
                // within the slice and comparing.
                let xplus = current_slice & !(current_slice << 1);
                let xminus = current_slice & !(current_slice >> 1);

                // Faces along +Y and -Y are found by comparing the current slice
                // with the slices directly above and below it.
                let yplus;
                if y < CHUNK_SIZE - 1 {
                    let upslice = self.chunk_mask[(y + 1) + z * CHUNK_SIZE];
                    yplus = current_slice & !upslice;
                } else { // on chunk border, all top faces are visible
                    yplus = current_slice;
                }

                let yminus;
                if y != 0 {
                    let downslice = self.chunk_mask[(y - 1) + z * CHUNK_SIZE];
                    yminus = current_slice & !downslice;
                } else { // on chunk border, all bottom faces are visible
                    yminus = current_slice;
                }

                // Faces along +Z and -Z are found by comparing the current slice
                // with the slices in front and behind it.
                let zplus;
                if z < CHUNK_SIZE - 1 {
                    let front_slice = self.chunk_mask[y + (z + 1) * CHUNK_SIZE];
                    zplus = current_slice & !front_slice;
                } else { // on chunk border, all front faces are visible
                    zplus = current_slice;
                }

                let zminus;
                if z > 0 {
                    let back_slice = self.chunk_mask[y + (z - 1) * CHUNK_SIZE];
                    zminus = current_slice & !back_slice;
                } else { // on chunk border, all back faces are visible
                    zminus = current_slice;
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

        let mut infos = Vec::<NormalGroupInfo>::new();

        let mut offset = 0;
        for i in 0..6 {
            let current_points = match i {
                0 => {
                    &points_right
                }
                1 => {
                    &points_left
                }
                2 => {
                    &points_top
                }
                3 => {
                    &points_bottom
                }
                4 => {
                    &points_front
                }
                5 => {
                    &points_back
                }
                _ => {
                    unreachable!("You shouldn't be here!")
                }
            };

            let info = NormalGroupInfo {
                count: current_points.len() as u32,
                offset,
                normal_id: i as u8,
            };

            println!("count: {}, offset: {}, normal_id: {}", info.count, info.offset, info.normal_id);

            offset += current_points.len() as u32;
            infos.push(info);
        }

        let mut points = Vec::<Vertex>::new();
        println!("points_right: {}", points_right.len());
        println!("points_left: {}", points_left.len());
        println!("points_top: {}", points_top.len());
        println!("points_bottom: {}", points_bottom.len());
        println!("points_front: {}", points_front.len());
        println!("points_back: {}", points_back.len());
        points.append(&mut points_right);
        points.append(&mut points_left);
        points.append(&mut points_top);
        points.append(&mut points_bottom);
        points.append(&mut points_front);
        points.append(&mut points_back);

        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Vertex Buffer"),
                contents: bytemuck::cast_slice(&points),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            },
        );

        let indirect_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Indirect Buffer"),
                contents: &[0; 120],
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            },
        );
        
        self.mesh = Some(ChunkMesh {
            cube_mesh: vertex_buffer,
            indirect_buffer: indirect_buffer,
        });

        self.infos = match infos.try_into() as Result<[NormalGroupInfo; 6], Vec<NormalGroupInfo>> {
            Ok(arr) => {
                Some(arr)
            },
            Err(v) => {
                println!("Conversion failed. Original Vec length was {} but expected 6.", v.len());
                None
            }
        };

        return true;
    }
}

pub struct ChunkMesh {
    cube_mesh: wgpu::Buffer,
    indirect_buffer: wgpu::Buffer,
}

impl ChunkMesh {
    pub fn get_instance_points(&self) -> &wgpu::Buffer {
        &self.cube_mesh
    }

    pub fn get_indirect_buffer(&self) -> &wgpu::Buffer {
        &self.indirect_buffer
    }
}