use engine_assets::AssetManager;
use image::DynamicImage;

use crate::constants::{MIP_LEVELS, TILE_SIZE_PIXELS};

pub struct TextureManager {
    pub block_array: LocalTexture,
    pub colormap_mask_array: LocalTexture,
    pub colormap_array: LocalTexture,
}

impl TextureManager {
    pub fn initialize(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        asset_manager: &AssetManager,
    ) -> TextureManager {
        TextureManager {
            block_array: Self::create_texture_array(
                device,
                queue,
                "Block Array",
                &asset_manager.block_textures,
                asset_manager.block_allocator.max_capacity(),
                TILE_SIZE_PIXELS,
                MIP_LEVELS,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::AddressMode::Repeat,
            ),
            colormap_mask_array: Self::create_texture_array(
                device,
                queue,
                "Mask Array",
                &asset_manager.block_colormap_mask_array,
                asset_manager.mask_allocator.max_capacity(),
                TILE_SIZE_PIXELS,
                1,
                wgpu::TextureFormat::R8Uint,
                wgpu::TextureSampleType::Uint,
                wgpu::AddressMode::Repeat,
            ),
            colormap_array: Self::create_texture_array(
                device,
                queue,
                "Colormap Array",
                &asset_manager.colormap_textures,
                asset_manager.colormap_allocator.max_capacity(),
                128,
                1,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::AddressMode::ClampToEdge,
            ),
        }
    }

    pub fn create_texture_array(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        images: &Vec<DynamicImage>,
        capacity: u32,
        size: u32,
        mip_level_count: u32,
        format: wgpu::TextureFormat,
        sample_type: wgpu::TextureSampleType,
        repeat_mode: wgpu::AddressMode,
    ) -> LocalTexture {
        let texture_size = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: capacity,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: texture_size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (i, img) in images.iter().enumerate() {
            let rgba = img.to_rgba8();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );

            // Generate Mips if requested x3
            if mip_level_count > 1 {
                for level in 1..mip_level_count {
                    let mip_size = (size >> level).max(1);
                    let resized = image::imageops::resize(
                        &rgba,
                        mip_size,
                        mip_size,
                        image::imageops::FilterType::Triangle,
                    );

                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: level,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: i as u32,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &resized,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * mip_size),
                            rows_per_image: Some(mip_size),
                        },
                        wgpu::Extent3d {
                            width: mip_size,
                            height: mip_size,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{}_view", label)),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: repeat_mode,
            address_mode_v: repeat_mode,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: if mip_level_count > 1 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{}_layout", label)),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("{}_bind_group", label)),
        });

        LocalTexture {
            bind_group,
            layout,
            dims: (size, size),
        }
    }
}

pub struct LocalTexture {
    pub bind_group: wgpu::BindGroup,
    pub layout: wgpu::BindGroupLayout,
    pub dims: (u32, u32),
}
