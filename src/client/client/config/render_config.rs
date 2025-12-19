use std::sync::atomic::AtomicU64;

static RENDER_CONFIG_ATOMIC: RenderConfigAtomic = RenderConfigAtomic {
    data: AtomicU64::new(0b01),
};

struct RenderConfigAtomic {
    data: AtomicU64,
}

pub struct RenderFlags;

impl RenderFlags {
    pub const CULL_FACES: u64 = 1 << 0;
    pub const LINE_RENDERING: u64 = 1 << 1;
}

pub struct RenderConfig {
    pub cull_chunk_faces: bool,
    pub use_line_rendering: bool,
}

impl RenderConfig {
    pub fn get_raw() -> Self {
        let raw = RENDER_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);

        RenderConfig {
            cull_chunk_faces: (raw & 0b1 << RenderFlags::CULL_FACES) != 0,
            use_line_rendering: (raw & 0b1 << RenderFlags::LINE_RENDERING) != 0,
        }
    }

    pub fn update_full(&self) {
        let mut raw = 0_u64;
        raw |= self.cull_chunk_faces as u64 >> RenderFlags::CULL_FACES;
        raw |= self.use_line_rendering as u64 >> RenderFlags::LINE_RENDERING;

        RENDER_CONFIG_ATOMIC
            .data
            .store(raw, std::sync::atomic::Ordering::SeqCst);
    }

    #[inline]
    fn set(mask: u64, value: bool) {
        RENDER_CONFIG_ATOMIC
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
    fn get(mask: u64) -> bool {
        let data = RENDER_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);
        (data & mask) != 0
    }

    #[inline]
    fn toggle(mask: u64) {
        RENDER_CONFIG_ATOMIC
            .data
            .fetch_xor(mask, std::sync::atomic::Ordering::SeqCst);
    }
}
