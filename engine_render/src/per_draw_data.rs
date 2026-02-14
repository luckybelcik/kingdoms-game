#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerDrawData {
    // the u32s might be an issue later cuz we cast from a u64
    // note from future self: as in it might overflow
    pub offset: u32,
    pub size: u32,
}
