use image::{DynamicImage, GenericImageView, GrayImage};

use crate::constants::{MIP_LEVELS, TILE_SIZE_PIXELS};

pub struct TextureManager {
    pub block_atlas: LocalTexture,
    pub colormap_mask_atlas: LocalTexture,
    pub colormap_array: LocalTexture,
}

impl TextureManager {
    pub fn initialize(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        block_atlas: &DynamicImage,
        mask_atlas: &GrayImage,
        colormaps: &Vec<DynamicImage>,
    ) -> TextureManager {
        TextureManager {
            block_atlas: Self::create_texture_with_tiled_mips(
                device,
                queue,
                "main_atlas",
                block_atlas,
                MIP_LEVELS,
                TILE_SIZE_PIXELS,
            ),
            colormap_mask_atlas: Self::create_mask_atlas_texture(device, queue, mask_atlas),
            colormap_array: Self::create_colormap_array_texture(device, queue, colormaps),
        }
    }

    fn create_texture_with_tiled_mips(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_name: &str,
        image: &DynamicImage,
        mip_levels: u32,
        grid_size: u32,
    ) -> LocalTexture {
        let rgba_image = image.to_rgba8();
        let dimensions = image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: mip_levels + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("block_texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        for i in 1..=mip_levels {
            let current_tile_size = (grid_size / (1 << i)).max(1);
            let mip_width = (dimensions.0 >> i).max(1);
            let mip_height = (dimensions.1 >> i).max(1);

            let mut mip_image = image::RgbaImage::new(mip_width, mip_height);

            for ty in 0..(dimensions.1 / grid_size) {
                for tx in 0..(dimensions.0 / grid_size) {
                    let tile = image.view(tx * grid_size, ty * grid_size, grid_size, grid_size);

                    let resized_tile = image::imageops::resize(
                        &tile.to_image(),
                        current_tile_size,
                        current_tile_size,
                        image::imageops::FilterType::Triangle,
                    );

                    image::imageops::replace(
                        &mut mip_image,
                        &resized_tile,
                        (tx * current_tile_size) as i64,
                        (ty * current_tile_size) as i64,
                    );
                }
            }

            let mip_texture_size = wgpu::Extent3d {
                width: mip_width,
                height: mip_height,
                depth_or_array_layers: 1,
            };

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: i,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip_image,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * mip_width),
                    rows_per_image: Some(mip_height),
                },
                mip_texture_size,
            );
        }

        let diffuse_texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some(&format!("{}_bind_group_layout", texture_name)),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some(&format!("{}_bind_group", texture_name)),
        });

        LocalTexture {
            bind_group,
            layout,
            dims: dimensions,
        }
    }

    pub fn create_mask_atlas_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mask_atlas: &GrayImage,
    ) -> LocalTexture {
        let dimensions = mask_atlas.dimensions();
        let mask_view = init_mask_atlas(device, queue, mask_atlas);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    // must be Uint because the format is R8Uint
                    sample_type: wgpu::TextureSampleType::Uint,
                },
                count: None,
            }],
            label: Some("mask_atlas_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&mask_view),
            }],
            label: Some("mask_atlas_bind_group"),
        });

        LocalTexture {
            bind_group,
            layout,
            dims: dimensions,
        }
    }

    pub fn create_colormap_array_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        images: &Vec<DynamicImage>,
    ) -> LocalTexture {
        let colormap_view = init_colormap_array(device, queue, images);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("colormap_array_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&colormap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("colormap_array_bind_group"),
        });

        LocalTexture {
            bind_group,
            layout,
            dims: (128, 128),
        }
    }
}

pub struct LocalTexture {
    pub bind_group: wgpu::BindGroup,
    pub layout: wgpu::BindGroupLayout,
    pub dims: (u32, u32),
}

fn init_mask_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mask_atlas: &GrayImage,
) -> wgpu::TextureView {
    let atlas_size = mask_atlas.dimensions().0;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Mask Atlas"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Uint, // Pure integers for our bit-packing
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        mask_atlas.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_size),
            rows_per_image: Some(atlas_size),
        },
        wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn init_colormap_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    images: &Vec<DynamicImage>,
) -> wgpu::TextureView {
    let layer_count = images.len().max(1) as u32;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Colormap Array"),
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    for (i, img) in images.iter().enumerate() {
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
            &img.as_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * 128),
                rows_per_image: Some(128),
            },
            wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
        );
    }

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
