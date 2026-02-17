use engine_core::entity_pos::EntityPos;
use image::DynamicImage;
use nalgebra_glm::Vec3;
use wgpu_buffer_allocator::allocator::SSBOAllocator;

use crate::{ChunkDrawCommand, gpu::Gpu, render_results::RenderResults, scene::Scene};

pub struct Renderer {
    pub gpu: Gpu,
    depth_texture_view: wgpu::TextureView,
    egui_renderer: egui_wgpu::Renderer,
    scene: Scene,
    pub chunk_ssbo: SSBOAllocator,
}

impl Renderer {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        atlas: &DynamicImage,
    ) -> Self {
        let gpu = Gpu::new_async(window, width, height).await;
        let depth_texture_view = gpu.create_depth_texture(width, height);

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.surface_config.format,
            egui_wgpu::RendererOptions {
                depth_stencil_format: Some(Self::DEPTH_FORMAT),
                msaa_samples: 1,
                ..Default::default()
            },
        );

        let chunk_ssbo = SSBOAllocator::new(&gpu.device, "Chunk SSBO", 134_217_728);

        let scene = Scene::new(
            &gpu.device,
            &gpu.queue,
            gpu.surface_format,
            chunk_ssbo.get_buffer(),
            atlas,
        );

        Self {
            gpu,
            depth_texture_view,
            egui_renderer,
            scene,
            chunk_ssbo,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(width, height);
    }

    pub fn render_frame(
        &mut self,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        camera_pos: &EntityPos,
        camera_rot: &Vec3,
        draw_commands: &Vec<ChunkDrawCommand>,
        textures_delta: egui::TexturesDelta,
    ) -> RenderResults {
        self.scene.update(&self.gpu.queue);

        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = match self.gpu.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Outdated) => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.surface_config);
                self.gpu
                    .surface
                    .get_current_texture()
                    .expect("Failed to get surface texture after reconfiguration!")
            }
            Err(error) => panic!("Failed to get surface texture: {:?}", error),
        };

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.gpu.surface_format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });

        encoder.insert_debug_marker("Render scene");

        let mut results: RenderResults;

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.19,
                            g: 0.24,
                            b: 0.42,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            results = self.scene.render(
                &mut render_pass,
                camera_pos,
                camera_rot,
                draw_commands,
                self.gpu.aspect_ratio(),
            );

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        results.allocated_blocks = self.chunk_ssbo.get_allocation_count() as u32;
        results.total_chunk_vram = self.chunk_ssbo.get_used_size();
        results.total_space = self.chunk_ssbo.get_size();
        results.free_space = self.chunk_ssbo.get_free_size();
        if results.allocated_blocks > 0 {
            results.avg_chunk_vram = results.total_chunk_vram / results.allocated_blocks as u64;
        }

        results
    }
}
