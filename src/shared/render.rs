use nalgebra_glm as glm;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    // first 15 bits is XYZ (5 bits per position)
    // the 3 bits after is the face normal ID
    // the next 10 bits are for the face size (RESERVED FOR GREEDY MESHING LATER)
    // the remaining 4 bits are unused
    pub data: u32,
    pub id: u32
}

impl Vertex {
    pub fn instance_description() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![1 => Uint32, 2 => Uint32];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBUTES,
        }
    }

    pub fn vertex_description() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawElementsIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

pub struct NormalGroupInfo {
    pub count: u32, 
    pub offset: u32,
    pub normal_id: u8,
}

pub fn create_mdi_commands(
    group_infos: &[NormalGroupInfo; 6], 
    camera_rot: glm::Vec3,
) -> [DrawElementsIndirectCommand; 6] {
    let mut commands: [DrawElementsIndirectCommand; 6] = [
        DrawElementsIndirectCommand {
            index_count: 6,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }; 6
    ];

    let pitch = camera_rot.x;
    let yaw = camera_rot.y;

    let x = yaw.cos() * pitch.cos();
    let y = pitch.sin();
    let z = yaw.sin() * pitch.cos();

    let camera_forward_vector = glm::vec3(x, y, z);

    let normal_vectors: [glm::Vec3; 6] = [
        glm::vec3(1.0, 0.0, 0.0),
        glm::vec3(-1.0, 0.0, 0.0),
        glm::vec3(0.0, 1.0, 0.0),
        glm::vec3(0.0, -1.0, 0.0),
        glm::vec3(0.0, 0.0, 1.0),
        glm::vec3(0.0, 0.0, -1.0),
    ];

    for i in 0..6 {
        let info = &group_infos[i];
        let normal_vector = normal_vectors[i];

        let dot_product = camera_forward_vector.dot(&normal_vector);

        let _is_culled = dot_product > 0.0;

        commands[i].first_instance = info.offset;

        // disabled for now
        if false {
            commands[i].instance_count = 0;
        } else {
            commands[i].instance_count = info.count;
        }
    }

    commands
}