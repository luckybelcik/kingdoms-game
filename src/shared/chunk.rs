use crate::shared::{constants::{CHUNK_SIZE, CHUNK_VOLUME}, render::{indirect::NormalGroupInfo, vertex::Vertex}};
use nalgebra_glm as glm;

pub struct Chunk {
    chunk_pos: glm::IVec3,
    blocks: [u16; CHUNK_VOLUME],
    pub mesh: Option<ChunkMesh>,
    pub infos: Option<[NormalGroupInfo; 6]>,
    is_dirty: bool,
}

impl Chunk {
    pub fn mark_dirt(&mut self) {
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
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            mesh: None,
            infos: None,
            is_dirty: true,
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE]
        }
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

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let block_id = self.get_block(x, y, z);

                    if block_id == 0 {
                        continue;
                    }

                    for (i, &normal) in normals.iter().enumerate() {
                        let nx = normal[0] as i32;
                        let ny = normal[1] as i32;
                        let nz = normal[2] as i32;

                        let neighbor_x = x as i32 + nx;
                        let neighbor_y = y as i32 + ny;
                        let neighbor_z = z as i32 + nz;
                        
                        let neighbor_block = 
                            if neighbor_x < 0 || neighbor_y < 0 || neighbor_z < 0 {
                                0 
                            } 
                            else {
                                self.get_block(
                                    neighbor_x as usize,
                                    neighbor_y as usize,
                                    neighbor_z as usize
                                )
                            };

                        if neighbor_block == 0 {
                            let x_u8 = (x & 0b11111) as u32;
                            let y_u8 = (y & 0b11111) as u32;
                            let z_u8 = (z & 0b11111) as u32;

                            let point = Vertex { data: x_u8 | (y_u8 << 5) | (z_u8 << 10) | ((i as u32) << 15), id: 1 };

                            match i {
                                0 => {
                                    points_right.push(point);
                                }
                                1 => {
                                    points_left.push(point);
                                }
                                2 => {
                                    points_top.push(point);
                                }
                                3 => {
                                    points_bottom.push(point);
                                }
                                4 => {
                                    points_front.push(point);
                                }
                                5 => {
                                    points_back.push(point);
                                }
                                _ => {
                                    unreachable!("You shouldn't be here!")
                                }
                            }
                        }
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