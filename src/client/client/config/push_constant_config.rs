use std::sync::atomic::AtomicU32;

static PUSH_CONSTANT_CONFIG_ATOMIC: PushConstantConfigAtomic = PushConstantConfigAtomic {
    data: AtomicU32::new(1),
};

struct PushConstantConfigAtomic {
    data: AtomicU32,
}

pub struct PushConstantFlags;

impl PushConstantFlags {
    pub const RENDER_TEXTURES: u32 = 1 << 0;
}

pub struct PushConstantConfig {
    pub render_textures: bool,
}

impl PushConstantConfig {
    pub fn get_full() -> Self {
        let raw = PUSH_CONSTANT_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);

        PushConstantConfig {
            render_textures: (raw & 0b1 << PushConstantFlags::RENDER_TEXTURES) != 0,
        }
    }

    pub fn get_raw() -> u32 {
        let raw = PUSH_CONSTANT_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);

        raw
    }

    pub fn update_full(&self) {
        let mut raw = 0_u32;
        raw |= self.render_textures as u32 >> PushConstantFlags::RENDER_TEXTURES;

        PUSH_CONSTANT_CONFIG_ATOMIC
            .data
            .store(raw, std::sync::atomic::Ordering::SeqCst);
    }

    #[inline]
    pub fn set(mask: u32, value: bool) {
        PUSH_CONSTANT_CONFIG_ATOMIC
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
    pub fn get(mask: u32) -> bool {
        let data = PUSH_CONSTANT_CONFIG_ATOMIC
            .data
            .load(std::sync::atomic::Ordering::SeqCst);
        (data & mask) != 0
    }

    #[inline]
    pub fn toggle(mask: u32) {
        PUSH_CONSTANT_CONFIG_ATOMIC
            .data
            .fetch_xor(mask, std::sync::atomic::Ordering::SeqCst);
    }
}
