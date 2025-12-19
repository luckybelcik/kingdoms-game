use std::sync::atomic::AtomicU64;

static MESH_CONFIG_ATOMIC: MeshConfigAtomic = MeshConfigAtomic {
    data: AtomicU64::new(1),
};

struct MeshConfigAtomic {
    data: AtomicU64,
}

pub struct MeshConfig {
    pub greedy_mesh: bool,
}

impl MeshConfig {
    pub fn get() -> Self {
        let raw = MESH_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);

        MeshConfig {
            greedy_mesh: (raw & 0b1 << 0) != 0,
        }
    }
}
