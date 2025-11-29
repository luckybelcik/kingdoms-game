use std::collections::HashMap;
use std::{sync::Arc};
use std::f32::consts::PI;
use web_time::{Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Theme, Window},
};

use crate::client::rendering::appinfo::AppInfo;
use crate::client::rendering::util::cast_ray_block_hit;
use crate::shared::constants::CHUNK_SIZE;
use crate::{client::rendering::renderer::Renderer, shared::{chunk::{Chunk}}};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    gui_state: Option<egui_winit::State>,
    pressed_keys: egui::ahash::HashSet<KeyCode>,
    pub chunks: HashMap<nalgebra_glm::IVec3, Chunk>,
    pub app_info: Option<AppInfo>,
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

        let mut app_info = AppInfo::default();

        {
            let inner_size = window_handle.inner_size();
            app_info.last_size = (inner_size.width, inner_size.height);
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
        app_info.last_render_time = Some(Instant::now());

        let mut chunks = HashMap::<nalgebra_glm::IVec3, Chunk>::new();
        const CHUNKS_SQUARED: i32 = 3;
        // first, generate chunks
        for i in 0..CHUNKS_SQUARED {
            for j in 0..CHUNKS_SQUARED {
                let chunk = Chunk::new_full(i, 0, j);
                chunks.insert(nalgebra_glm::vec3(i, 0, j), chunk);
            }
        }

        // then generate meshes
        if let Some(renderer) = &self.renderer {
            for i in 0..CHUNKS_SQUARED {
                for j in 0..CHUNKS_SQUARED {
                    let pos = nalgebra_glm::vec3(i, 0, j);
                    if let Some(mut chunk) = chunks.remove(&pos) {
                        (&mut chunk).generate_mesh(&renderer.get_gpu().device);
                        chunks.insert(nalgebra_glm::vec3(i, 0, j), chunk);
                    }
                }
            }
        }

        self.chunks = chunks;

        app_info.camera_pos = nalgebra_glm::vec3(-10.0, 5.0, -10.0);
        app_info.camera_rot = nalgebra_glm::vec3(0.0, 0.0, 0.0);

        self.app_info = Some(app_info);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(app_info) = self.app_info.as_mut() else { return };

        app_info.tick += 1;

        let total: f32 = (app_info.delta_history.iter().sum::<u16>()) as f32;
        let avg_delta_time = total / (app_info.delta_history.len() as f32);

        if app_info.tick % 10 == 0 {
            let now = Instant::now();
            if let Some(last) = app_info.last_render_time {
                let delta_time = now - last;
                app_info.delta_history.push_back(delta_time.as_millis() as u16);

                if app_info.delta_history.len() > 512 {
                    app_info.delta_history.pop_front();
                }

                if avg_delta_time != 0.0 {
                    app_info.avg_fps_history.push_back((1000.0 / avg_delta_time) as u16);

                    if app_info.avg_fps_history.len() > 128 {
                        app_info.avg_fps_history.pop_front();
                    }
                }
            }
        }

        let mut lowest_fps = 0;
        let mut highest_fps = 0;

        if !app_info.avg_fps_history.is_empty() {
            lowest_fps = *app_info.avg_fps_history.iter().min().unwrap_or(&0);
            highest_fps = *app_info.avg_fps_history.iter().max().unwrap_or(&0);
        }

        {
            let (Some(gui_state), Some(window)) = (
                self.gui_state.as_mut(),
                self.window.as_ref(),
            ) else {
                return;
            };

            if gui_state.on_window_event(window, &event).consumed {
                return;
            }
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event, event_loop);
            }
            WindowEvent::Resized(size) => self.handle_resize(size),
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw(avg_delta_time, highest_fps, lowest_fps);
            }
            _ => (),
        }

        let Some(window) = self.window.as_ref() else { return };
        window.request_redraw();
    }
}

impl App {
    fn handle_keyboard_input(&mut self, event: KeyEvent, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let PhysicalKey::Code(key_code) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    self.pressed_keys.insert(key_code);
                }
                ElementState::Released => {
                    self.pressed_keys.remove(&key_code);
                }
            }
        }
        if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
            event_loop.exit();
        }
        if let PhysicalKey::Code(KeyCode::Comma) = event.physical_key {
            let app_info = self.app_info.as_ref().unwrap();

            if let Some((chunk_pos, (x, y, z))) = cast_ray_block_hit(app_info.camera_pos, app_info.camera_rot, &self.chunks) {
                if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
                    chunk.set_block(x, y, z, 0);
                }
            }
        }
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        let (Some(renderer), Some(app_info)) = (self.renderer.as_mut(), self.app_info.as_mut()) else { return };

        if new_size.width > 0 && new_size.height > 0 {
            log::info!("Resizing renderer surface to: ({}, {})", new_size.width, new_size.height);
            renderer.resize(new_size.width, new_size.height);
            app_info.last_size = (new_size.width, new_size.height);
        }
    }

    fn handle_redraw(&mut self, avg_delta_time: f32, highest_fps: u16, lowest_fps: u16) {
        const TICK_RATE: u32 = 60;
        const FIXED_TIMESTEP: f64 = 1.0 / TICK_RATE as f64;

        {
            let mut accumulator;
            let delta_time;
            let now;

            // we have to do the client tick in this scope to respect borrowing rules
            {
                let Some(app_info) = self.app_info.as_mut() else { return };
                now = Instant::now();
                delta_time = now - app_info.last_render_time.unwrap();

                accumulator = app_info.accumulator;
                accumulator += delta_time.as_secs_f64();

                while accumulator >= FIXED_TIMESTEP {
                    self.handle_client_tick(FIXED_TIMESTEP as f32);
                    accumulator -= FIXED_TIMESTEP;
                }
            }

            let Some(app_info) = self.app_info.as_mut() else { return };
            app_info.accumulator = accumulator;
            app_info.last_render_time = Some(now);

            self.update_camera(delta_time.as_secs_f32());
        }

        let (Some(gui_state), Some(renderer), Some(window), Some(app_info)) = (
            self.gui_state.as_mut(),
            self.renderer.as_mut(),
            self.window.as_ref(),
            self.app_info.as_mut(),
        ) else {
            return;
        };

        let gui_input = gui_state.take_egui_input(window);
        gui_state.egui_ctx().begin_pass(gui_input);

        draw_ui(
            gui_state.egui_ctx(),
            avg_delta_time,
            highest_fps,
            lowest_fps,
            app_info,
        );

        let egui::FullOutput {
            textures_delta,
            shapes,
            pixels_per_point,
            platform_output,
            ..
        } = gui_state.egui_ctx().end_pass();

        gui_state.handle_platform_output(window, platform_output);

        let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

        let screen_descriptor = {
            let (width, height) = app_info.last_size;
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
            &mut self.chunks,
            app_info.camera_pos,
            app_info.camera_rot,
        );
    }

    fn handle_client_tick(&mut self, _delta_seconds: f32) {
        let Some(app_info) = self.app_info.as_mut() else {
            return;
        };

        app_info.chunk_count = self.chunks.len() as u64;

        if app_info.chunk_count > 0 && app_info.total_chunk_vram > 0 {
            app_info.chunk_count = self.chunks.len() as u64;
            app_info.avg_chunk_vram = app_info.total_chunk_vram / app_info.chunk_count as u64;
        }
    }

    fn update_camera(&mut self, delta_seconds: f32) {
        let Some(app_info) = self.app_info.as_mut() else { return };

        let move_speed = 20.0 * delta_seconds;
        let rotation_speed = 2.0 * delta_seconds;

        // Rotation
        if self.pressed_keys.contains(&KeyCode::ArrowUp) {
            app_info.camera_rot.x += rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowDown) {
            app_info.camera_rot.x -= rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowLeft) {
            app_info.camera_rot.y += rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowRight) {
            app_info.camera_rot.y -= rotation_speed;
        }
        // Clamp pitch
        app_info.camera_rot.x = app_info.camera_rot.x.clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);

        // Movement
        let (sin_y, cos_y) = app_info.camera_rot.y.sin_cos();
        if self.pressed_keys.contains(&KeyCode::KeyW) {
            app_info.camera_pos.x += cos_y * move_speed;
            app_info.camera_pos.z += sin_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            app_info.camera_pos.x -= cos_y * move_speed;
            app_info.camera_pos.z -= sin_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            app_info.camera_pos.x -= sin_y * move_speed;
            app_info.camera_pos.z += cos_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            app_info.camera_pos.x += sin_y * move_speed;
            app_info.camera_pos.z -= cos_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::Space) {
            app_info.camera_pos.y += move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ShiftLeft) {
            app_info.camera_pos.y -= move_speed;
        }
    }
}

fn draw_ui(
    ctx: &egui::Context,
    avg_delta_time: f32,
    highest_fps: u16,
    lowest_fps: u16,
    app_info: &mut AppInfo,
) {
    let title = "Rust/Wgpu";
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
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
                ui.label(egui::RichText::new(title).color(egui::Color32::LIGHT_GREEN));
                ui.separator();
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("v0.1.0").color(egui::Color32::ORANGE));
                ui.separator();
            });
        });
    });

    egui::SidePanel::left("left").show(ctx, |ui| {
        ui.heading("Scene Tree");
    });

    egui::SidePanel::right("right").show(ctx, |ui| {
        ui.heading("Performance");

        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            let color = if avg_delta_time < 12.0 {
                egui::Color32::GREEN
            } else if avg_delta_time < 16.7 {
                egui::Color32::ORANGE
            } else {
                egui::Color32::RED
            };
            ui.label(egui::RichText::new(format!("delta: {:.1} ms", avg_delta_time)).color(color));
            if avg_delta_time > 0.0 {
                ui.label(egui::RichText::new(format!("FPS: {:.1}", 1000.0 / avg_delta_time)).color(color));
            }
            ui.label(egui::RichText::new(format!("Highest FPS: {}", highest_fps)).color(color));
            ui.label(egui::RichText::new(format!("Lowest FPS: {}", lowest_fps)).color(color));
        });

        ui.heading("Debug Info");

        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            ui.label(egui::RichText::new(format!("window size: {}x {}y", app_info.last_size.0, app_info.last_size.1)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("chunk vram footprint: {}", app_info.total_chunk_vram)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("memory per chunk: {}", app_info.avg_chunk_vram)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new("warning, the memory amount above takes both vertices and indices into consideration").color(egui::Color32::GRAY).small());
            ui.label(egui::RichText::new(format!("chunks: {}", app_info.chunk_count)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("chunks update count: {}", app_info.chunk_updates)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("cam pos: {:.2}, {:.2}, {:.2}", app_info.camera_pos.x, app_info.camera_pos.y, app_info.camera_pos.z)).color(egui::Color32::LIGHT_BLUE));
            ui.label(egui::RichText::new(format!("cam rot: {:.2}, {:.2}", app_info.camera_rot.x.to_degrees(), app_info.camera_rot.y.to_degrees())).color(egui::Color32::LIGHT_BLUE));
        });
    });

    egui::TopBottomPanel::bottom("Console").show(ctx, |ui| {
        ui.heading("Console");
    });
}