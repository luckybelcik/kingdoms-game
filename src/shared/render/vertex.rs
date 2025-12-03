#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    // first 15 bits is XYZ (5 bits per position)
    // the 3 bits after is the face normal ID
    // the next 10 bits are for the face size (RESERVED FOR GREEDY MESHING LATER)
    // the remaining 4 bits are unused
    pub data: u32,
    // theoretically all bits are used,
    // but in reality i only care for the 16 first bits
    // might use the rest for texture variation stuff
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
}