use std::sync::atomic::AtomicU64;

static MESH_CONFIG_ATOMIC: MeshConfigAtomic = MeshConfigAtomic {
    data: AtomicU64::new(1),
};

struct MeshConfigAtomic {
    data: AtomicU64,
}

pub struct MeshFlags;

impl MeshFlags {
    pub const GREEDY_MESH: u64 = 1 << 0;
}

pub struct MeshConfig {
    pub greedy_mesh: bool,
}

impl MeshConfig {
    pub fn get_full() -> Self {
        let raw = MESH_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);

        MeshConfig {
            greedy_mesh: (raw & 0b1 << 0) != 0,
        }
    }

    #[inline]
    pub fn set(mask: u64, value: bool) {
        MESH_CONFIG_ATOMIC
            .data
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |current_data| {
                    let next_data = if value {
                        current_data | mask
                    } else {
                        current_data & !mask
                    };

                    Some(next_data)
                },
            )
            .ok();
    }

    #[inline]
    pub fn get(mask: u64) -> bool {
        let data = MESH_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);
        (data & mask) != 0
    }

    #[inline]
    pub fn toggle(mask: u64) {
        MESH_CONFIG_ATOMIC
            .data
            .fetch_xor(mask, std::sync::atomic::Ordering::SeqCst);
    }
}
