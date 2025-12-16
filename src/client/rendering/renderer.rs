use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use arc_swap::ArcSwap;
use wgpu_buffer_allocator::allocator::SSBOAllocator;

use crate::client::rendering::{
    apprenderconfig::AppRenderConfig,
    chunk_mesh::{MeshJob, SendableChunkMesh},
    client_chunk::ClientChunk,
    core::Scene,
    gpu::Gpu,
    render_results::RenderResults,
};

pub struct Renderer {
    gpu: Gpu,
    depth_texture_view: wgpu::TextureView,
    egui_renderer: egui_wgpu::Renderer,
    scene: Scene,
    chunk_ssbo: SSBOAllocator,
    job_sender: std::sync::mpsc::Sender<MeshJob>,
    mesh_receiver: std::sync::mpsc::Receiver<SendableChunkMesh>,
}

impl Renderer {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn get_gpu(&self) -> &Gpu {
        &self.gpu
    }

    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
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

        let scene = Scene::new(&gpu.device, gpu.surface_format, chunk_ssbo.get_buffer());

        let (mesh_job_tx, mesh_job_rx) = std::sync::mpsc::channel::<MeshJob>();
        let (mesh_tx, mesh_rx) = std::sync::mpsc::channel::<SendableChunkMesh>();

        std::thread::spawn(move || {
            for job in mesh_job_rx {
                let sendable = SendableChunkMesh::make_mesh(&job);
                mesh_tx.send(sendable).unwrap();
            }
        });

        Self {
            gpu,
            depth_texture_view,
            egui_renderer,
            scene,
            chunk_ssbo,
            job_sender: mesh_job_tx,
            mesh_receiver: mesh_rx,
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
        textures_delta: egui::TexturesDelta,
        chunks_mut: &mut HashMap<nalgebra_glm::IVec3, ArcSwap<ClientChunk>>,
        dirty_chunks: &mut HashSet<nalgebra_glm::IVec3>,
        camera_pos: nalgebra_glm::Vec3,
        camera_rot: nalgebra_glm::Vec3,
        render_config: &AppRenderConfig,
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

        let dirty_keys = dirty_chunks;

        for key in dirty_keys.iter() {
            if let Some(chunk) = chunks_mut.get(key) {
                let chunk_pos_right = nalgebra_glm::vec3(key.x + 1, key.y, key.z);
                let chunk_pos_left = nalgebra_glm::vec3(key.x - 1, key.y, key.z);
                let chunk_pos_up = nalgebra_glm::vec3(key.x, key.y + 1, key.z);
                let chunk_pos_down = nalgebra_glm::vec3(key.x, key.y - 1, key.z);
                let chunk_pos_forward = nalgebra_glm::vec3(key.x, key.y, key.z + 1);
                let chunk_pos_backward = nalgebra_glm::vec3(key.x, key.y, key.z - 1);

                let nearby_chunks: [Option<Arc<ClientChunk>>; 6] = [
                    chunks_mut.get(&chunk_pos_right).map(|c| c.load_full()),
                    chunks_mut.get(&chunk_pos_left).map(|c| c.load_full()),
                    chunks_mut.get(&chunk_pos_up).map(|c| c.load_full()),
                    chunks_mut.get(&chunk_pos_down).map(|c| c.load_full()),
                    chunks_mut.get(&chunk_pos_forward).map(|c| c.load_full()),
                    chunks_mut.get(&chunk_pos_backward).map(|c| c.load_full()),
                ];

                let loaded_chunk = chunk.load_full();

                self.job_sender
                    .send((loaded_chunk, nearby_chunks, render_config.clone()))
                    .unwrap();
            }
        }

        dirty_keys.clear();

        for sent_mesh in self.mesh_receiver.try_iter() {
            let chunk = chunks_mut.get(&sent_mesh.pos);
            if let Some(existing_chunk) = chunk {
                let mut new_chunk = (*(existing_chunk.load_full())).clone();
                new_chunk
                    .mesh
                    .update_mesh(&self.gpu.queue, &mut self.chunk_ssbo, &sent_mesh);
                existing_chunk.store(Arc::new(new_chunk));
            }
        }

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
                chunks_mut,
                camera_rot,
                camera_pos,
                self.gpu.aspect_ratio(),
                render_config,
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
