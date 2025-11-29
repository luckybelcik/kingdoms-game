use std::collections::HashMap;
use std::{sync::Arc};
use std::f32::consts::PI;
use egui::Align2;
use web_time::{Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Theme, Window},
};

use crate::client::rendering::appinfo::AppInfo;
use crate::client::rendering::apprenderconfig::AppRenderConfig;
use crate::client::rendering::render_results::RenderResults;
use crate::client::rendering::ui_state::{PopupWindow, RenderConfigData, UIState, WorldSizePopupData};
use crate::client::rendering::util::{cast_ray_block_hit, cast_ray_block_before};
use crate::shared::render::vertex::Vertex;
use crate::{client::rendering::renderer::Renderer, shared::{chunk::{Chunk}}};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    gui_state: Option<egui_winit::State>,
    pressed_keys: egui::ahash::HashSet<KeyCode>,
    pub chunks: HashMap<nalgebra_glm::IVec3, Chunk>,
    pub app_info: AppInfo,
    pub app_render_config: AppRenderConfig,
    render_results: RenderResults,
    ui_state: UIState,
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

        self.app_info = app_info;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.app_info.tick += 1;

        let total: f32 = (self.app_info.delta_history.iter().sum::<u16>()) as f32;
        let avg_delta_time = total / (self.app_info.delta_history.len() as f32);

        if self.app_info.tick % 10 == 0 {
            let now = Instant::now();
            if let Some(last) = self.app_info.last_render_time {
                let delta_time = now - last;
                self.app_info.delta_history.push_back(delta_time.as_millis() as u16);

                if self.app_info.delta_history.len() > 512 {
                    self.app_info.delta_history.pop_front();
                }

                if avg_delta_time != 0.0 {
                    self.app_info.avg_fps_history.push_back((1000.0 / avg_delta_time) as u16);

                    if self.app_info.avg_fps_history.len() > 128 {
                        self.app_info.avg_fps_history.pop_front();
                    }
                }
            }
        }

        let mut lowest_fps = 0;
        let mut highest_fps = 0;

        if !self.app_info.avg_fps_history.is_empty() {
            lowest_fps = *self.app_info.avg_fps_history.iter().min().unwrap_or(&0);
            highest_fps = *self.app_info.avg_fps_history.iter().max().unwrap_or(&0);
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
            if event.state == ElementState::Pressed {
                if let Some((chunk_pos, (x, y, z))) = cast_ray_block_hit(self.app_info.camera_pos, self.app_info.camera_rot, &self.chunks) {
                    if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
                        log::info!("Break block at {} {} {}", x, y, z);
                        chunk.set_block(x, y, z, 0);
                    }
                }
            }
        }
        if let PhysicalKey::Code(KeyCode::Period) = event.physical_key {
            if event.state == ElementState::Pressed {
                if let Some((chunk_pos, (x, y, z))) = cast_ray_block_before(self.app_info.camera_pos, self.app_info.camera_rot, &self.chunks) {
                    if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
                        log::info!("Place block at {} {} {}", x, y, z);
                        chunk.set_block(x, y, z, 1);
                    }
                }
            }
        }
        if let PhysicalKey::Code(KeyCode::KeyP) = event.physical_key {
            if event.state == ElementState::Pressed {
                self.app_render_config.toggle_render_textures_bit();
            }
        }
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        let Some(renderer) = self.renderer.as_mut() else { return };

        if new_size.width > 0 && new_size.height > 0 {
            log::info!("Resizing renderer surface to: ({}, {})", new_size.width, new_size.height);
            renderer.resize(new_size.width, new_size.height);
            self.app_info.last_size = (new_size.width, new_size.height);
        }
    }

    fn handle_redraw(&mut self, avg_delta_time: f32, highest_fps: u16, lowest_fps: u16) {
        const TICK_RATE: u32 = 60;
        const FIXED_TIMESTEP: f64 = 1.0 / TICK_RATE as f64;

        let now = Instant::now();
        let delta_time = now - self.app_info.last_render_time.unwrap();

        let mut accumulator = self.app_info.accumulator;
        accumulator += delta_time.as_secs_f64();

        while accumulator >= FIXED_TIMESTEP {
            self.handle_client_tick(FIXED_TIMESTEP as f32);
            accumulator -= FIXED_TIMESTEP;
        }

        self.app_info.accumulator = accumulator;
        self.app_info.last_render_time = Some(now);

        self.update_camera(delta_time.as_secs_f32());

        let gui_input;
        {
            if let (Some(gui_state), Some(window)) = (self.gui_state.as_mut(), self.window.as_mut()) {
                gui_input = gui_state.take_egui_input(window);
                gui_state.egui_ctx().begin_pass(gui_input);
            } else {
                return;
            }
        }

        draw_ui(
            self,
            avg_delta_time,
            highest_fps,
            lowest_fps,
        );

        let (Some(gui_state), Some(renderer), Some(window)) = (
            self.gui_state.as_mut(),
            self.renderer.as_mut(),
            self.window.as_ref(),
        ) else {
            return;
        };

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
            let (width, height) = self.app_info.last_size;
            if width == 0 || height == 0 {
                return;
            }
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [width, height],
                pixels_per_point,
            }
        };

        self.render_results = renderer.render_frame(
            screen_descriptor,
            paint_jobs,
            textures_delta,
            &mut self.chunks,
            self.app_info.camera_pos,
            self.app_info.camera_rot,
            &self.app_render_config,
        );
    }

    fn handle_client_tick(&mut self, _delta_seconds: f32) {
        self.app_info.chunk_count = self.chunks.len() as u64;

        if self.app_info.chunk_count > 0 && self.app_info.total_chunk_vram > 0 {
            self.app_info.chunk_count = self.chunks.len() as u64;
            self.app_info.avg_chunk_vram = self.app_info.total_chunk_vram / self.app_info.chunk_count as u64;
        }
    }

    fn update_camera(&mut self, delta_seconds: f32) {
        let move_speed = 20.0 * delta_seconds;
        let rotation_speed = 2.0 * delta_seconds;

        // Rotation
        if self.pressed_keys.contains(&KeyCode::ArrowUp) {
            self.app_info.camera_rot.x += rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowDown) {
            self.app_info.camera_rot.x -= rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowLeft) {
            self.app_info.camera_rot.y += rotation_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowRight) {
            self.app_info.camera_rot.y -= rotation_speed;
        }
        // Clamp pitch
        self.app_info.camera_rot.x = self.app_info.camera_rot.x.clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);

        // Movement
        let (sin_y, cos_y) = self.app_info.camera_rot.y.sin_cos();
        if self.pressed_keys.contains(&KeyCode::KeyW) {
            self.app_info.camera_pos.x += cos_y * move_speed;
            self.app_info.camera_pos.z += sin_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            self.app_info.camera_pos.x -= cos_y * move_speed;
            self.app_info.camera_pos.z -= sin_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            self.app_info.camera_pos.x -= sin_y * move_speed;
            self.app_info.camera_pos.z += cos_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            self.app_info.camera_pos.x += sin_y * move_speed;
            self.app_info.camera_pos.z -= cos_y * move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::Space) {
            self.app_info.camera_pos.y += move_speed;
        }
        if self.pressed_keys.contains(&KeyCode::ShiftLeft) {
            self.app_info.camera_pos.y -= move_speed;
        }
    }
}

fn draw_ui(
    app: &mut App,
    avg_delta_time: f32,
    highest_fps: u16,
    lowest_fps: u16,
) {
    let ctx = app.gui_state.as_mut().unwrap().egui_ctx();
    let title = "Rust/Wgpu";
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("World", |ui| {
                    if ui.button("Regenerate Chunks").clicked() {
                        ui.close();
                    }
                    if ui.button("Change World Size").clicked() {
                        app.ui_state.toggle_popup(PopupWindow::WorldSize(WorldSizePopupData::default()));
                        ui.close();
                    }
                });
                ui.separator();

                if ui.button("Render Config").clicked() {
                    app.ui_state.toggle_popup(PopupWindow::RenderConfig(RenderConfigData::new(&app.app_render_config)));
                }

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

        let memory_per_chunk = if app.render_results.chunk_count > 0 {
            app.render_results.triangles_rendered * std::mem::size_of::<Vertex>() as u32 / app.render_results.chunk_count
        } else {
            0
        };

        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            ui.label(egui::RichText::new(format!("window size: {}x {}y", app.app_info.last_size.0, app.app_info.last_size.1)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("total triangles: {}", app.render_results.triangles_rendered)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("chunk vram footprint: {}", app.render_results.triangles_rendered * std::mem::size_of::<Vertex>() as u32)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("memory per chunk: {}", memory_per_chunk)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new("warning, the memory amount above takes both vertices and indices into consideration").color(egui::Color32::GRAY).small());
            ui.label(egui::RichText::new(format!("chunks: {}", app.render_results.chunk_count)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("chunks update count: {}", app.app_info.chunk_updates)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("draw calls: {}", app.render_results.draw_calls)).color(egui::Color32::ORANGE));
            ui.label(egui::RichText::new(format!("cam pos: {:.2}, {:.2}, {:.2}", app.app_info.camera_pos.x, app.app_info.camera_pos.y, app.app_info.camera_pos.z)).color(egui::Color32::LIGHT_BLUE));
            ui.label(egui::RichText::new(format!("cam rot: {:.2}, {:.2}", app.app_info.camera_rot.x.to_degrees(), app.app_info.camera_rot.y.to_degrees())).color(egui::Color32::LIGHT_BLUE));
        });
    });

    egui::TopBottomPanel::bottom("Console").show(ctx, |ui| {
        ui.heading("Console");
    });

    let mut state_to_set_to: Option<PopupWindow> = None;

    match &mut app.ui_state.popup_window {
        PopupWindow::None => {},
        PopupWindow::WorldSize(popup_data) => {
            egui::Window::new("World Size").anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO).resizable(false).collapsible(false).show(ctx, |ui| {
                ui.add(egui::DragValue::new(&mut popup_data.size).prefix("Chunk area: ").range(0..=32).clamp_existing_to_range(true));

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.button("Save").clicked() {
                            state_to_set_to = Some(PopupWindow::None);
                        }
                        if ui.button("Close").clicked() {
                            state_to_set_to = Some(PopupWindow::None);
                        }
                    });
                });
            });
        }
        PopupWindow::RenderConfig(popup_data) => {
            egui::Window::new("Render Config").anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO).resizable(false).collapsible(false).show(ctx, |ui| {
                ui.checkbox(&mut popup_data.render_textures, "Block Visuals");

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.button("Save").clicked() {
                            app.app_render_config.set_render_textures_bit(popup_data.render_textures);
                            state_to_set_to = Some(PopupWindow::None);
                        }
                        if ui.button("Close").clicked() {
                            state_to_set_to = Some(PopupWindow::None);
                        }
                    });
                });
            });
        }
    }

    if let Some(state) = state_to_set_to {
        app.ui_state.popup_window = state;
    }
}