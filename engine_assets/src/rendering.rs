use std::{path::Path, sync::Arc};

use lasso::ThreadedRodeo;

use crate::{
    colormap_registry::{ColormapRegistry, string_to_source_id},
    manifest::{ColormapConfig, FaceConfigWithVariants, SourceValue},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureMetadata {
    // two first have 11 bits, last has 10 bits
    pub packed_colormap_ids: u32,
    pub mask_atlas_id: u32, // 0 if no mask
    // each ids has 5 bits, 2 bits for texture flipping (bit 31 is X bit 32 is Y)
    pub packed_source_ids_and_flipbits: u32,
    // 1st bit = use texture variants, 2nd bit = use colormap mask variants
    // last 16 bits = index into additional mask array
    pub additional_meta: u32,
}

// bits 0-10: CM0_ID, 11-21: CM1_ID, 22-31: CM2_ID
pub fn pack_colormap_ids(
    config: &FaceConfigWithVariants,
    registry: &ColormapRegistry,
    ns_path: &Path,
    interner: &Arc<ThreadedRodeo>,
) -> u32 {
    let get_id = |conf: &Option<ColormapConfig>| {
        conf.as_ref()
            .map(|c| registry.get_colormap_id(&c.map, ns_path, interner))
            .unwrap_or(0)
    };

    let id0 = get_id(&config.colormap0) & 0x7FF;
    let id1 = get_id(&config.colormap1) & 0x7FF;
    let id2 = get_id(&config.colormap2) & 0x3FF;

    id0 | (id1 << 11) | (id2 << 22)
}

pub fn pack_sources(config: &FaceConfigWithVariants) -> u32 {
    let get_pair = |conf: &Option<ColormapConfig>| -> (u32, u32) {
        if let Some(c) = conf {
            match &c.source {
                SourceValue::Single(s) => (string_to_source_id(s) & 0x1F, 0),
                SourceValue::Dual(s1, s2) => (
                    string_to_source_id(s1) & 0x1F,
                    string_to_source_id(s2) & 0x1F,
                ),
            }
        } else {
            (0, 0)
        }
    };

    let (s0a, s0b) = get_pair(&config.colormap0);
    let (s1a, s1b) = get_pair(&config.colormap1);
    let (s2a, s2b) = get_pair(&config.colormap2);

    let flip_x = config.flip_x.unwrap_or(false) as u32;
    let flip_y = config.flip_y.unwrap_or(false) as u32;
    s0a | (s0b << 5)
        | (s1a << 10)
        | (s1b << 15)
        | (s2a << 20)
        | (s2b << 25)
        | (flip_x << 30)
        | (flip_y << 31)
}
