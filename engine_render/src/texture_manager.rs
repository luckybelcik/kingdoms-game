use image::{DynamicImage, GenericImageView};

use crate::constants::{MIP_LEVELS, TILE_SIZE_PIXELS};

pub struct TextureManager {
    main_atlas: LocalTexture,
}

impl TextureManager {
    pub fn initialize(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: &DynamicImage,
    ) -> TextureManager {
        TextureManager {
            main_atlas: Self::create_texture_with_tiled_mips(
                device,
                queue,
                "main_atlas",
                atlas,
                MIP_LEVELS,
                TILE_SIZE_PIXELS,
            ),
        }
    }

    pub fn get_main_atlas_bind_group(&self) -> &wgpu::BindGroup {
        &self.main_atlas.bind_group
    }

    pub fn get_main_atlas_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.main_atlas.layout
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

        LocalTexture { bind_group, layout }
    }
}

pub struct LocalTexture {
    pub bind_group: wgpu::BindGroup,
    pub layout: wgpu::BindGroupLayout,
}
