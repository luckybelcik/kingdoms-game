pub struct AssetSlopConfig {
    pub block_padding: u32,
    pub mask_padding: u32,
    pub colormap_padding: u32,
}

impl Default for AssetSlopConfig {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        return Self {
            block_padding: 32,
            mask_padding: 16,
            colormap_padding: 4,
        };

        #[cfg(not(debug_assertions))]
        return Self {
            block_padding: 0,
            mask_padding: 0,
            colormap_padding: 0,
        };
    }
}
