use std::collections::VecDeque;
use std::{sync::Arc};
use std::f32::consts::PI;
use web_time::{Instant};
use winit::{keyboard::KeyCode,
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    keyboard::{PhysicalKey},
    window::{Theme, Window},
};

use crate::{client::rendering::renderer::Renderer, shared::{chunk::{Chunk}, render::Vertex}};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    gui_state: Option<egui_winit::State>,
    last_render_time: Option<Instant>,
    last_size: (u32, u32),
    pressed_keys: egui::ahash::HashSet<KeyCode>,
    pub chunks: Vec<Chunk>,
    chunk_updates: u64,
    pub camera_pos: nalgebra_glm::Vec3,
    pub camera_rot: nalgebra_glm::Vec3,
    delta_history: VecDeque<u16>,
    avg_fps_history: VecDeque<u16>,
    tick: u128,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attributes = Window::default_attributes();

        {
            attributes = attributes.with_title("Standalone Winit/Wgpu Example");
        }

        let Ok(window) = event_loop.create_window(attributes) else {
            return;
        };

        let first_window_handle = self.window.is_none();
        let window_handle = Arc::new(window);
        self.window = Some(window_handle.clone());
        if !first_window_handle {
            return;
        }
        let gui_context = egui::Context::default();

        {
            let inner_size = window_handle.inner_size();
            self.last_size = (inner_size.width, inner_size.height);
        }

        let viewport_id = gui_context.viewport_id();
        let gui_state = egui_winit::State::new(
            gui_context,
            viewport_id,
            &window_handle,
            Some(window_handle.scale_factor() as _),
            Some(Theme::Dark),
            None,
        );

        let (width, height) = (
            window_handle.inner_size().width,
            window_handle.inner_size().height,
        );

        {
            env_logger::init();
            let renderer = pollster::block_on(async move {
                Renderer::new(window_handle.clone(), width, height).await
            });
            self.renderer = Some(renderer);
        }

        self.gui_state = Some(gui_state);
        self.last_render_time = Some(Instant::now());

        let mut chunks = Vec::new();
        if let Some(renderer) = &self.renderer {
            for i in 0..1 {
                for j in 0..1 {
                    let mut chunk = Chunk::new_full(i, 0, j);
                    // if it returns true (which it does when the mesh was regenerated) then we increment the chunk update counter
                    if chunk.generate_mesh(&renderer.get_gpu().device) {
                        self.chunk_updates += 1;
                    }
                    chunks.push(chunk);
                }
            }
        }

        self.chunks = chunks;

        self.camera_pos = nalgebra_glm::vec3(-10.0, 5.0, -10.0);
        self.camera_rot = nalgebra_glm::vec3(0.0, 0.0, 0.0);
    }

    

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.tick += 1;

        let triangle_count = self.get_triangles_sent();
        let total_chunk_vram = self.get_total_chunk_memory();
        let single_chunk_vram = self.get_total_chunk_memory() / self.chunks.len() as u64;
        let chunk_count = self.chunks.len();
        let chunk_update_count = self.chunk_updates;

        let total: f32 = (self.delta_history.iter().sum::<u16>()) as f32;
        let avg_delta_time = total / (self.delta_history.len() as f32);

        if self.tick % 10 == 0 {
            let now = Instant::now();
            if let Some(last) = self.last_render_time {
                let delta_time = now - last;
                self.delta_history.push_back(delta_time.as_millis() as u16);

                if self.delta_history.len() > 512 {
                    self.delta_history.pop_front();
                }

                if avg_delta_time != 0.0 {
                    self.avg_fps_history.push_back((1000.0 / avg_delta_time) as u16);

                    if self.avg_fps_history.len() > 128 {
                        self.avg_fps_history.pop_front();
                    }
                }
            }
        }

        let mut lowest_fps = 0;
        let mut highest_fps = 0;

        if self.delta_history.len() > 0 {
            lowest_fps = *self.avg_fps_history.iter().min().unwrap();
            highest_fps = *self.avg_fps_history.iter().max().unwrap();
        }

        let (Some(gui_state), Some(renderer), Some(window), Some(last_render_time)) = (
            self.gui_state.as_mut(),
            self.renderer.as_mut(),
            self.window.as_ref(),
            self.last_render_time.as_mut(),
        ) else {
            return;
        };

        if gui_state.on_window_event(window, &event).consumed {
            return;
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent { physical_key, state, .. },
                ..
            } => {
                if let PhysicalKey::Code(key_code) = physical_key {
                    match state {
                        winit::event::ElementState::Pressed => {
                            self.pressed_keys.insert(key_code);
                        }
                        winit::event::ElementState::Released => {
                            self.pressed_keys.remove(&key_code);
                        }
                    }
                }
                if let PhysicalKey::Code(KeyCode::Escape) = physical_key {
                    event_loop.exit();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                {
                    let scale_factor = window.scale_factor() as f32;
                    gui_state.egui_ctx().set_pixels_per_point(scale_factor);
                }
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if width == 0 || height == 0 {
                    return;
                }

                log::info!("Resizing renderer surface to: ({width}, {height})");
                renderer.resize(width, height);
                self.last_size = (width, height);

                {
                    let scale_factor = window.scale_factor() as f32;
                    gui_state.egui_ctx().set_pixels_per_point(scale_factor);
                }
            }
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now - *last_render_time;
                *last_render_time = now;

                // --- Camera Update ---
                let delta_seconds = delta_time.as_secs_f32();
                let move_speed = 20.0 * delta_seconds;
                let rotation_speed = 2.0 * delta_seconds;

                // Rotation
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::ArrowUp) {
                    self.camera_rot.x += rotation_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::ArrowDown) {
                    self.camera_rot.x -= rotation_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::ArrowLeft) {
                    self.camera_rot.y += rotation_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::ArrowRight) {
                    self.camera_rot.y -= rotation_speed;
                }
                // Clamp pitch
                self.camera_rot.x = self.camera_rot.x.clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);

                // Movement
                let (sin_y, cos_y) = self.camera_rot.y.sin_cos();
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::KeyW) {
                    self.camera_pos.x += cos_y * move_speed;
                    self.camera_pos.z += sin_y * move_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::KeyS) {
                    self.camera_pos.x -= cos_y * move_speed;
                    self.camera_pos.z -= sin_y * move_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::KeyA) {
                    self.camera_pos.x -= sin_y * move_speed;
                    self.camera_pos.z += cos_y * move_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::KeyD) {
                    self.camera_pos.x += sin_y * move_speed;
                    self.camera_pos.z -= cos_y * move_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::Space) {
                    self.camera_pos.y += move_speed;
                }
                if self.pressed_keys.contains(&winit::keyboard::KeyCode::ShiftLeft) {
                    self.camera_pos.y -= move_speed;
                }

                let gui_input = gui_state.take_egui_input(window);

                gui_state.egui_ctx().begin_pass(gui_input);

                let title = "Rust/Wgpu";

                {
                    egui::TopBottomPanel::top("top").show(gui_state.egui_ctx(), |ui| {
                        ui.horizontal(|ui| {
                            egui::MenuBar::new().ui(ui, |ui| {
                                ui.menu_button("File", |ui| {
                                    if ui.button("Load").clicked() {
                                        ui.close();
                                    }
                                    if ui.button("Save").clicked() {
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Import").clicked() {
                                        ui.close();
                                    }
                                });

                                ui.menu_button("Edit", |ui| {
                                    if ui.button("Clear").clicked() {
                                        ui.close();
                                    }
                                    if ui.button("Reset").clicked() {
                                        ui.close();
                                    }
                                });

                                ui.separator();

                                ui.label(
                                    egui::RichText::new(title).color(egui::Color32::LIGHT_GREEN),
                                );

                                ui.separator();
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new("v0.1.0").color(egui::Color32::ORANGE),
                                );
                                ui.separator();
                            });
                        });
                    });

                    egui::SidePanel::left("left").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Scene Tree");
                    });

                    egui::SidePanel::right("right").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Performance");

                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            let color = if delta_time.as_millis() < 12 {
                                egui::Color32::GREEN
                            } else if delta_time.as_millis() < 16 {
                                egui::Color32::ORANGE
                            } else {
                                egui::Color32::RED
                            };

                            ui.label(
                                egui::RichText::new(format!("delta: {:.1} ms", avg_delta_time)).color(color),
                            );

                            ui.label(
                                egui::RichText::new(format!("FPS: {:.1}", 1000.0 / avg_delta_time)).color(color),
                            );

                            ui.label(
                                egui::RichText::new(format!("Lowest FPS: {}", highest_fps)).color(color),
                            );

                            ui.label(
                                egui::RichText::new(format!("Highest FPS: {}", lowest_fps)).color(color),
                            );
                        });

                        ui.heading("Debug Info");

                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.label(
                                egui::RichText::new(format!("window size: {}x {}y", self.last_size.0, self.last_size.1)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new(format!("triangles sent: {}", triangle_count)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new(format!("chunk vram footprint: {}", total_chunk_vram)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new(format!("memory per chunk: {}", single_chunk_vram)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new("warning, the memory amount above takes both vertices and indices into consideration").color(egui::Color32::GRAY).small(),
                            );

                            ui.label(
                                egui::RichText::new(format!("chunks: {}", chunk_count)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new(format!("chunks update count: {}", chunk_update_count)).color(egui::Color32::ORANGE),
                            );

                            ui.label(
                                egui::RichText::new(format!("cam pos: {:.2}, {:.2}, {:.2}", self.camera_pos.x, self.camera_pos.y, self.camera_pos.z)).color(egui::Color32::LIGHT_BLUE),
                            );

                            ui.label(
                                egui::RichText::new(format!("cam rot: {:.2}, {:.2}", self.camera_rot.x.to_degrees(), self.camera_rot.y.to_degrees())).color(egui::Color32::LIGHT_BLUE),
                            );
                        });
                    });

                    egui::TopBottomPanel::bottom("Console").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Console");
                    });
                }

                let egui_winit::egui::FullOutput {
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    platform_output,
                    ..
                } = gui_state.egui_ctx().end_pass();

                gui_state.handle_platform_output(window, platform_output);

                let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

                let screen_descriptor = {
                    let (width, height) = self.last_size;
                    if width == 0 || height == 0 {
                        return;
                    }
                    egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point,
                    }
                };

                renderer.render_frame(
                    screen_descriptor,
                    paint_jobs,
                    textures_delta,
                    delta_time,
                    &mut self.chunks,
                    self.camera_pos, self.camera_rot,
                );
            }
            _ => (),
        }

        window.request_redraw();
    }
}

impl App {
    fn get_triangles_sent(&self) -> u64 {
        let mut triangles_sent = 0;
        for chunk in &self.chunks {
            if let Some(mesh) = &chunk.mesh {
                triangles_sent += mesh.get_instance_points().size() / std::mem::size_of::<Vertex>() as u64;
            }
        } 

        triangles_sent
    }

    fn get_total_chunk_memory(&self) -> u64 {
        let mut mem = 0;
        for chunk in &self.chunks {
            if let Some(mesh) = &chunk.mesh {
                mem += mesh.get_instance_points().size() as u64;
            }
        } 

        mem
    }
}