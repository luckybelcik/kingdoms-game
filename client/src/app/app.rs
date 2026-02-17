use egui::{Align2, Color32};
use engine_assets::AssetManager;
use engine_core::entity_pos::EntityPos;
use engine_net::{
    client_actions::{ClientKeybindableActions, PlayerActions},
    client_packet::{ClientAction, ClientPacket},
    player_id::PlayerId,
};
use engine_render::{ChunkDrawCommand, render_results::RenderResults, renderer::Renderer};
use engine_settings::client_config::{
    mesh_config::{MeshConfig, MeshFlags},
    push_constant_config::{PushConstantConfig, PushConstantFlags},
    render_config::{RenderConfig, RenderFlags},
};
use nalgebra_glm::Vec3;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Theme, Window},
};

use crate::{
    app::{
        app_actions::AppKeybindableActions,
        appinfo::AppInfo,
        input_handler::{ActionOption, InputHandler},
        ui_state::{PopupWindow, RenderConfigData, UIState, WorldSizePopupData},
    },
    client::client::Client,
    connection_details::ClientConnectionType,
};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    asset_manager: AssetManager,
    gui_state: Option<egui_winit::State>,
    pressed_keys: egui::ahash::HashSet<KeyCode>,
    pub app_info: AppInfo,
    render_results: RenderResults,
    ui_state: UIState,
    client: Option<Client>,
    input_handler: Option<InputHandler>,
    scheduled_client_bindable_actions: Vec<ClientKeybindableActions>,
    start_time: Option<Instant>,
}

impl App {
    pub fn new(player_id: PlayerId, connection_type: ClientConnectionType) -> Self {
        Self {
            window: None,
            renderer: None,
            asset_manager: AssetManager::init(),
            gui_state: None,
            pressed_keys: Default::default(),
            app_info: Default::default(),
            render_results: Default::default(),
            ui_state: Default::default(),
            client: Some(Client::create(player_id, connection_type)),
            input_handler: Some(InputHandler::new()),
            scheduled_client_bindable_actions: Vec::new(),
            start_time: Some(Instant::now()),
        }
    }
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

        let atlas = &self.asset_manager.atlas;
        let texture_mapping = &self.asset_manager.texture_mapping_table;

        {
            env_logger::init();
            let renderer = pollster::block_on(async move {
                Renderer::new(window_handle.clone(), width, height, atlas, texture_mapping).await
            });
            self.renderer = Some(renderer);
        }

        self.gui_state = Some(gui_state);
        app_info.last_render_time = Some(Instant::now());

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

        if self.app_info.tick.is_multiple_of(10) {
            let now = Instant::now();
            if let Some(last) = self.app_info.last_render_time {
                let delta_time: Duration = now - last;
                self.app_info
                    .delta_history
                    .push_back(delta_time.as_millis() as u16);

                if self.app_info.delta_history.len() > 512 {
                    self.app_info.delta_history.pop_front();
                }

                if avg_delta_time != 0.0 {
                    self.app_info
                        .avg_fps_history
                        .push_back((1000.0 / avg_delta_time) as u16);

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
            let (Some(gui_state), Some(window)) = (self.gui_state.as_mut(), self.window.as_ref())
            else {
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

        let Some(window) = self.window.as_ref() else {
            return;
        };
        window.request_redraw();
    }
}

impl App {
    fn handle_keyboard_input(
        &mut self,
        event: KeyEvent,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        if let PhysicalKey::Code(key_code) = event.physical_key
            && event.state == ElementState::Pressed
        {
            if let Some(input_handler) = &self.input_handler {
                let action = input_handler.handle_input(&key_code);

                if action.is_single_press() {
                    match action {
                        ActionOption::App(app_action) => {
                            self.handle_single_press_app_action(&app_action, &event_loop);
                        }
                        ActionOption::Client(client_action) => {
                            if let Some(client) = &mut self.client {
                                client.handle_single_press_client_action(&client_action);
                            }
                        }
                        ActionOption::None => (),
                    }
                }
            }
        }

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
    }

    fn handle_single_press_app_action(
        &mut self,
        action: &AppKeybindableActions,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        match action {
            AppKeybindableActions::ExitApp => {
                event_loop.exit();
            }
            AppKeybindableActions::ToggleTextureRendering => {
                PushConstantConfig::toggle(PushConstantFlags::RENDER_TEXTURES);
            }
            AppKeybindableActions::ToggleLineRendering => {
                RenderConfig::toggle(RenderFlags::LINE_RENDERING);
            }
            AppKeybindableActions::ToggleDebugUI => {
                self.ui_state.show_ui = !self.ui_state.show_ui;
            }
            AppKeybindableActions::ForceCrash => {
                panic!("Forced crash!");
            }
        }
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        if new_size.width > 0 && new_size.height > 0 {
            log::info!(
                "Resizing renderer surface to: ({}, {})",
                new_size.width,
                new_size.height
            );
            renderer.resize(new_size.width, new_size.height);
            self.app_info.last_size = (new_size.width, new_size.height);
        }
    }

    fn handle_redraw(&mut self, avg_delta_time: f32, highest_fps: u16, lowest_fps: u16) {
        const TICK_RATE: u32 = 20;
        const FIXED_TIMESTEP: f64 = 1.0 / TICK_RATE as f64;

        for key_code in &self.pressed_keys {
            if let Some(input_handler) = &self.input_handler {
                let action = input_handler.handle_input(&key_code);

                if action.is_holdable() {
                    match action {
                        ActionOption::App(_) => (),
                        ActionOption::Client(client_action) => {
                            self.scheduled_client_bindable_actions.push(client_action);
                        }
                        ActionOption::None => (),
                    }
                }
            }
        }

        let now = Instant::now();
        let delta_time = now - self.app_info.last_render_time.unwrap();

        if let Some(client) = &mut self.client {
            client.handle_tickless_actions(
                &mut self.scheduled_client_bindable_actions,
                delta_time.as_secs_f32(),
            );
        }

        let mut accumulator = self.app_info.accumulator;
        accumulator += delta_time.as_secs_f64();

        while accumulator >= FIXED_TIMESTEP {
            if let Some(client) = &mut self.client {
                client.handle_client_tick(
                    &mut self.app_info,
                    &mut self.scheduled_client_bindable_actions,
                    FIXED_TIMESTEP as f32,
                );

                self.scheduled_client_bindable_actions.clear();
            }
            accumulator -= FIXED_TIMESTEP;
        }

        self.app_info.accumulator = accumulator;
        self.app_info.last_render_time = Some(now);

        let gui_input;
        {
            if let (Some(gui_state), Some(window)) = (self.gui_state.as_mut(), self.window.as_mut())
            {
                gui_input = gui_state.take_egui_input(window);
                gui_state.egui_ctx().begin_pass(gui_input);
            } else {
                return;
            }
        }

        if self.ui_state.show_ui {
            draw_ui(self, avg_delta_time, highest_fps, lowest_fps);
        }

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

        let mut draw_commands = Vec::new();
        let mut camera_pos = EntityPos::new(0.0, 0.0, 0.0);
        let mut camera_rot = Vec3::new(0.0, 0.0, 0.0);

        match &mut self.client {
            Some(client) => {
                client.purge_unused_meshes(&renderer.gpu.queue, &mut renderer.chunk_ssbo);
                client.update_meshes(&renderer.gpu.queue, &mut renderer.chunk_ssbo);

                for client_chunk in &client.client_world.chunks {
                    let mesh = client_chunk.1.mesh.load();

                    let culled_calls =
                        mesh.get_visible_draw_calls(client.camera_pos, *client_chunk.0);

                    draw_commands.push(ChunkDrawCommand {
                        chunk_pos: *client_chunk.0,
                        draw_call_info: culled_calls,
                    });
                }

                camera_pos = client.camera_pos;
                camera_rot = client.camera_rot;
            }
            _ => (),
        }

        self.render_results = renderer.render_frame(
            screen_descriptor,
            paint_jobs,
            &camera_pos,
            &camera_rot,
            &draw_commands,
            textures_delta,
            self.start_time.unwrap().elapsed().as_secs_f32(),
        );
    }
}

fn draw_ui(app: &mut App, avg_delta_time: f32, highest_fps: u16, lowest_fps: u16) {
    let mut selected_block_string = "";
    let mut selected_block_id = 0;
    let block_list = app.asset_manager.block_registry.get_all_blocks();

    if let Some(client) = &app.client {
        if let Some(player_data) = client.get_plater_data() {
            selected_block_id = player_data.selected_block;
            selected_block_string = block_list
                .iter()
                .find(|(_, id)| **id + 1 == selected_block_id)
                .map(|(name, _)| name.as_str())
                .unwrap_or("Unknown");
        }
    }

    let mut new_block = selected_block_id;
    let mut new_block_string = selected_block_string;

    let ctx = app.gui_state.as_mut().unwrap().egui_ctx();
    let mode;
    let mode_color;

    if let Some(client) = &mut app.client {
        #[cfg(debug_assertions)]
        {
            mode = "Debug";
            mode_color = egui::Color32::RED;
        }
        #[cfg(not(debug_assertions))]
        {
            mode = "Release";
            mode_color = egui::Color32::GREEN;
        }

        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("World", |ui| {
                        if ui.button("Regenerate Chunks").clicked() {
                            ui.close();
                        }
                        if ui.button("Regenerate Meshes").clicked() {
                            client.dirty_chunks =
                                client.client_world.chunks.keys().cloned().collect();
                            ui.close();
                        }
                        if ui.button("Change World Size").clicked() {
                            app.ui_state.toggle_popup(PopupWindow::WorldSize(
                                WorldSizePopupData::default(),
                            ));
                            ui.close();
                        }
                    });
                    ui.separator();

                    if ui.button("Render Config").clicked() {
                        app.ui_state
                            .toggle_popup(PopupWindow::RenderConfig(RenderConfigData::new()));
                    }

                    ui.separator();

                    ui.label(egui::RichText::new(mode).color(mode_color));

                    ui.separator();

                    ui.label(egui::RichText::new(format!("{} / {}", os, arch)));

                    ui.separator();
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .color(egui::Color32::ORANGE),
                    );
                    ui.separator();
                });
            });
        });

        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.heading("Scene Tree");
            ui.separator();
            ui.heading("Block Picker");

            egui::ComboBox::from_label("Select Block")
                .selected_text(selected_block_string)
                .show_ui(ui, |ui| {
                    for (name, id) in block_list {
                        if ui
                            .selectable_value(&mut selected_block_id, *id + 1, name)
                            .clicked()
                        {
                            new_block = *id + 1;
                            new_block_string = name;
                        };
                    }
                });

            ui.separator();
            ui.label(format!("Current ID: {}", selected_block_id));
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
                ui.label(
                    egui::RichText::new(format!("delta: {:.1} ms", avg_delta_time)).color(color),
                );
                if avg_delta_time > 0.0 {
                    ui.label(
                        egui::RichText::new(format!("FPS: {:.1}", 1000.0 / avg_delta_time))
                            .color(color),
                    );
                }
                ui.label(egui::RichText::new(format!("Highest FPS: {}", highest_fps)).color(color));
                ui.label(egui::RichText::new(format!("Lowest FPS: {}", lowest_fps)).color(color));
            });

            ui.heading("Debug Info");

            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "window size: {}x {}y",
                        app.app_info.last_size.0, app.app_info.last_size.1
                    ))
                    .color(egui::Color32::ORANGE),
                );
                ui.label(
                    egui::RichText::new(format!("chunks: {}", app.render_results.chunk_count))
                        .color(egui::Color32::ORANGE),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "chunks update count: {}",
                        app.app_info.chunk_updates
                    ))
                    .color(egui::Color32::ORANGE),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "cam pos: {:.2}, {:.2}, {:.2}",
                        client.camera_pos.x, client.camera_pos.y, client.camera_pos.z
                    ))
                    .color(egui::Color32::ORANGE),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "cam rot: {:.2}, {:.2}",
                        client.camera_rot.x.to_degrees(),
                        client.camera_rot.y.to_degrees()
                    ))
                    .color(egui::Color32::ORANGE),
                );
            });

            ui.heading("Memory Usage");

            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "ssbo allocations: {}",
                        app.render_results.allocated_blocks
                    ))
                    .color(egui::Color32::LIGHT_BLUE),
                );
                ui.label(
                    egui::RichText::new(format!("ssbo size: {}", app.render_results.total_space))
                        .color(egui::Color32::LIGHT_BLUE),
                );
                if app.render_results.total_space > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "free ssbo memory: {:.1}%",
                            app.render_results.free_space as f32
                                / app.render_results.total_space as f32
                                * 100.0
                        ))
                        .color(egui::Color32::LIGHT_BLUE),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "used ssbo memory: {:.1}%",
                            app.render_results.total_chunk_vram as f32
                                / app.render_results.total_space as f32
                                * 100.0
                        ))
                        .color(egui::Color32::LIGHT_BLUE),
                    );
                }
                ui.label(
                    egui::RichText::new(format!(
                        "average mem per chunk: {}",
                        app.render_results.avg_chunk_vram
                    ))
                    .color(egui::Color32::LIGHT_BLUE),
                );
            });

            ui.heading("Render Info");

            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "triangles rendered: {}",
                        app.render_results.triangles_rendered
                    ))
                    .color(egui::Color32::LIGHT_GREEN),
                );
                ui.label(
                    egui::RichText::new(format!("draw calls: {}", app.render_results.draw_calls))
                        .color(egui::Color32::LIGHT_GREEN),
                );
            });
        });

        egui::TopBottomPanel::bottom("Console").show(ctx, |ui| {
            ui.heading("Console");
        });

        let mut state_to_set_to: Option<PopupWindow> = None;

        match &mut app.ui_state.popup_window {
            PopupWindow::None => {}
            PopupWindow::WorldSize(popup_data) => {
                egui::Window::new("World Size")
                    .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.add(
                            egui::DragValue::new(&mut popup_data.size)
                                .prefix("Chunk area: ")
                                .range(0..=32)
                                .clamp_existing_to_range(true),
                        );

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
                egui::Window::new("Render Config")
                    .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new("Push Constant Config")
                                .size(10.0)
                                .color(Color32::GOLD),
                        );
                        ui.checkbox(&mut popup_data.render_textures, "Block Visuals");

                        ui.label(
                            egui::RichText::new("Bool Config")
                                .size(10.0)
                                .color(Color32::GOLD),
                        );
                        ui.checkbox(&mut popup_data.cull_chunk_faces, "Cull Chunk Faces");

                        ui.label(
                            egui::RichText::new("Meshing Config")
                                .size(10.0)
                                .color(Color32::GOLD),
                        );
                        ui.checkbox(&mut popup_data.greedy_meshing, "Greedy Meshing");

                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                if ui.button("Save").clicked() {
                                    PushConstantConfig::set(
                                        PushConstantFlags::RENDER_TEXTURES,
                                        popup_data.render_textures,
                                    );
                                    RenderConfig::set(
                                        RenderFlags::CULL_FACES,
                                        popup_data.cull_chunk_faces,
                                    );
                                    MeshConfig::set(
                                        MeshFlags::GREEDY_MESH,
                                        popup_data.greedy_meshing,
                                    );
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

        if let Some(client) = &mut app.client {
            if let Some(player_data) = client.get_plater_data_mut() {
                player_data.selected_block = new_block;
            }

            client.send_packet(ClientPacket {
                player_id: client.get_player_id(),
                action: ClientAction::PlayerAction(PlayerActions::ChangeSelectedBlock(
                    new_block_string.to_string(),
                )),
            });
        }
    }
}
