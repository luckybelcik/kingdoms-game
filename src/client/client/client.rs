use crate::{
    client::{
        app::appinfo::AppInfo,
        client::{
            chunk_mesh::StoredChunkMesh,
            client_actions::{ClientKeybindableActions, PlayerActions},
            client_chunk::ClientChunk,
            packet_serializer::PacketSerializer,
        },
        connection_details::ClientConnectionType,
    },
    shared::{
        communication::{
            client_packet::{ClientAction, ClientPacket},
            player_data::ClientPlayerData,
            player_id::PlayerId,
            server_packet::ServerPacket,
        },
        coordinate_systems::entity_pos::EntityPos,
        util::raycast::cast_ray,
    },
};
use std::{f32::consts::PI, sync::Arc};

use arc_swap::ArcSwap;
use nalgebra_glm::{Vec3, vec3};
use rustc_hash::{FxHashMap, FxHashSet};
use wgpu_buffer_allocator::allocator::SSBOAllocator;

use crate::{client::client::mesher::Mesher, shared::coordinate_systems::chunk_pos::ChunkPos};

pub struct Client {
    pub chunks: FxHashMap<ChunkPos, ClientChunk>,
    pub dirty_chunks: FxHashSet<ChunkPos>,
    mesher: Mesher,
    serializer: PacketSerializer,
    pub camera_pos: EntityPos,
    pub camera_rot: Vec3,
    player_data: Option<ClientPlayerData>,
    player_id: PlayerId,
    connection_type: ClientConnectionType,
    tick: u128,
}

impl Client {
    pub fn create(player_id: PlayerId, connection_type: ClientConnectionType) -> Self {
        Client {
            chunks: FxHashMap::default(),
            dirty_chunks: FxHashSet::default(),
            mesher: Mesher::create(),
            serializer: PacketSerializer::create(),
            camera_pos: EntityPos::new(0.0, 0.0, 0.0),
            camera_rot: vec3(0.0, 0.0, 0.0),
            player_data: None,
            player_id,
            connection_type,
            tick: 0,
        }
    }

    pub fn get_player_id(&self) -> PlayerId {
        self.player_id.clone()
    }

    pub fn handle_tickless_actions(
        &mut self,
        scheduled_actions: &mut Vec<ClientKeybindableActions>,
        delta_time: f32,
    ) {
        for action in scheduled_actions {
            if action.is_tickrate_independent() {
                self.handle_holdable_client_action(action, delta_time);
            }
        }
    }

    pub fn handle_client_tick(
        &mut self,
        app_info: &mut AppInfo,
        scheduled_actions: &mut Vec<ClientKeybindableActions>,
        delta_time: f32,
    ) {
        app_info.chunk_count = self.chunks.len() as u64;

        if app_info.chunk_count > 0 && app_info.total_chunk_vram > 1 {
            app_info.chunk_count = self.chunks.len() as u64;
            app_info.avg_chunk_vram = app_info.total_chunk_vram / app_info.chunk_count;
        }

        let mut packets = Vec::new();

        for action in scheduled_actions {
            if !action.is_tickrate_independent() {
                self.handle_holdable_client_action(action, delta_time);
            }
        }

        if self.tick.is_multiple_of(19) {
            if let Some(data) = &self.player_data {
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::DebugCheckSync(data.clone()),
                });
            }
        }

        match &self.connection_type {
            ClientConnectionType::Local(details) => {
                while let Ok(server_packet) = details.server_packet_receiver.try_recv() {
                    packets.push(server_packet);
                }
            }
            ClientConnectionType::Remote(_) => {
                unimplemented!("Remote connection logic not implemented");
            }
        }

        for packet in packets {
            self.serializer.deserialize_server_packet_bytes(packet);
        }

        for deserialized in self.serializer.receive_finished_tasks() {
            self.receive_packet(deserialized);
        }

        self.mesher
            .upload_for_remeshing(&mut self.dirty_chunks, &mut self.chunks);

        self.tick += 1;
    }

    pub fn update_meshes(&mut self, queue: &wgpu::Queue, allocator: &mut SSBOAllocator) {
        let meshes = self.mesher.receive_from_remeshing();

        for mesh in meshes {
            if let Some(client_chunk) = self.chunks.get(&mesh.pos) {
                let arc_mesh = client_chunk.mesh.load_full();
                let mut new_mesh = (*arc_mesh).clone();
                new_mesh.update_mesh(queue, allocator, &mesh);
                client_chunk.mesh.store(Arc::new(new_mesh));
            }
        }
    }

    pub fn receive_packet(&mut self, server_packet: ServerPacket) {
        match server_packet {
            ServerPacket::Ping => {
                // nothing bruh
            }
            ServerPacket::Chunk(chunk) => {
                let mesh = StoredChunkMesh::new_empty();
                let pos = chunk.get_chunk_pos();
                let client_chunk = ClientChunk::new_prewrapped(
                    ArcSwap::new(Arc::from(chunk)),
                    ArcSwap::new(Arc::new(mesh)),
                );
                self.chunks.insert(pos, client_chunk);
                self.dirty_chunks.insert(pos);
                self.dirty_chunks.insert(pos.offset_copy(1, 0, 0));
                self.dirty_chunks.insert(pos.offset_copy(-1, 0, 0));
                self.dirty_chunks.insert(pos.offset_copy(0, 1, 0));
                self.dirty_chunks.insert(pos.offset_copy(0, -1, 0));
                self.dirty_chunks.insert(pos.offset_copy(0, 0, 1));
                self.dirty_chunks.insert(pos.offset_copy(0, 0, -1));
            }
            ServerPacket::PlayerData(data) => {
                if let Some(current_data) = self.player_data.as_ref() {
                    if current_data != &data {
                        eprintln!("Client and server desynced.");
                        ClientPlayerData::log_desync(&current_data, &data);
                    }
                } else {
                    println!("Player data was None; accepting new data from server");
                    self.camera_pos = data.position.clone();
                    self.player_data = Some(data);
                }
            }
            ServerPacket::DebugPlayer(data) => {
                println!("Player debug data: {:?}", data);
            }
            ServerPacket::DebugChunk(data) => {
                println!("Chunk debug data: {:?}", data);
            }
            ServerPacket::Denial(reason) => {
                println!("Packet denied: {}", reason.message())
            }
        }
    }

    pub fn send_packet(&mut self, client_packet: ClientPacket) {
        match &self.connection_type {
            ClientConnectionType::Local(details) => {
                details
                    .client_packet_sender
                    .send(bincode::serialize(&client_packet).unwrap())
                    .unwrap();
            }
            ClientConnectionType::Remote(_) => {
                unimplemented!(
                    "Remote connection packet sending from clietn not implmentednd no no no!"
                )
            }
        }
    }

    pub fn handle_single_press_client_action(&mut self, action: &ClientKeybindableActions) {
        if action.is_holdable() {
            return;
        }

        let player_id = self.player_id.clone();

        match action {
            ClientKeybindableActions::BreakBlock => {
                self.send_packet(ClientPacket {
                    player_id: self.player_id.clone(),
                    action: ClientAction::PlayerAction(PlayerActions::BreakBlock(
                        self.camera_rot,
                        self.camera_pos,
                    )),
                });
                if let Some(raycast_result) =
                    cast_ray(self.camera_pos, self.camera_rot, &self.chunks, 64)
                {
                    if let Some(client_chunk) = self.chunks.get_mut(&raycast_result.hit.0) {
                        client_chunk.chunk.rcu(|old_chunk| {
                            let mut new_chunk = (**old_chunk).clone();
                            new_chunk.set_block(raycast_result.hit.1, 0, &mut self.dirty_chunks);
                            Arc::new(new_chunk)
                        });
                    }
                }
            }
            ClientKeybindableActions::PlaceBlock => {
                self.send_packet(ClientPacket {
                    player_id: self.player_id.clone(),
                    action: ClientAction::PlayerAction(PlayerActions::PlaceBlock(
                        self.camera_rot,
                        self.camera_pos,
                    )),
                });
                if let Some(raycast_result) =
                    cast_ray(self.camera_pos, self.camera_rot, &self.chunks, 64)
                {
                    if let Some(client_chunk) = self.chunks.get_mut(&raycast_result.previous.0) {
                        client_chunk.chunk.rcu(|old_chunk| {
                            let mut new_chunk = (**old_chunk).clone();
                            new_chunk.set_block(
                                raycast_result.previous.1,
                                1,
                                &mut self.dirty_chunks,
                            );
                            Arc::new(new_chunk)
                        });
                    }
                }
            }
            ClientKeybindableActions::MoveForwards => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::MoveBackwards => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::MoveLeft => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::MoveRight => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::MoveUp => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::MoveDown => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::RotateUp => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::RotateDown => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::RotateLeft => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::RotateRight => {
                unreachable!("Action not single press");
            }
            ClientKeybindableActions::RequestServerPlayerData => {
                self.send_packet(ClientPacket {
                    player_id,
                    action: ClientAction::DebugPlayer,
                });
            }
            ClientKeybindableActions::RequestServerChunkInfo => {
                self.send_packet(ClientPacket {
                    player_id,
                    action: ClientAction::DebugChunks,
                });
            }
        }
    }

    pub fn handle_holdable_client_action(
        &mut self,
        action: &ClientKeybindableActions,
        delta_seconds: f32,
    ) {
        if action.is_single_press() {
            return;
        }

        let move_speed = 10.0 * delta_seconds;
        let rotation_speed = 1.0 * delta_seconds;
        let (sin_y, cos_y) = self.camera_rot.y.sin_cos();

        match action {
            ClientKeybindableActions::BreakBlock => {
                unreachable!("Action not holdable");
            }
            ClientKeybindableActions::PlaceBlock => {
                unreachable!("Action not holdable");
            }
            ClientKeybindableActions::MoveForwards => {
                self.camera_pos.x += cos_y * move_speed;
                self.camera_pos.z += sin_y * move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.x += cos_y * move_speed;
                    player_data.position.z += sin_y * move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveForwards(
                        self.camera_rot,
                    )),
                });
            }
            ClientKeybindableActions::MoveBackwards => {
                self.camera_pos.x -= cos_y * move_speed;
                self.camera_pos.z -= sin_y * move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.x -= cos_y * move_speed;
                    player_data.position.z -= sin_y * move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveBackwards(
                        self.camera_rot,
                    )),
                });
            }
            ClientKeybindableActions::MoveLeft => {
                self.camera_pos.x -= sin_y * move_speed;
                self.camera_pos.z += cos_y * move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.x -= sin_y * move_speed;
                    player_data.position.z += cos_y * move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveLeft(self.camera_rot)),
                });
            }
            ClientKeybindableActions::MoveRight => {
                self.camera_pos.x += sin_y * move_speed;
                self.camera_pos.z -= cos_y * move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.x += sin_y * move_speed;
                    player_data.position.z -= cos_y * move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveRight(self.camera_rot)),
                });
            }
            ClientKeybindableActions::MoveUp => {
                self.camera_pos.y += move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.y += move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveUp),
                });
            }
            ClientKeybindableActions::MoveDown => {
                self.camera_pos.y -= move_speed;
                if let Some(player_data) = &mut self.player_data {
                    player_data.position.y -= move_speed;
                }
                self.send_packet(ClientPacket {
                    player_id: self.player_id,
                    action: ClientAction::PlayerAction(PlayerActions::MoveDown),
                });
            }
            ClientKeybindableActions::RotateUp => {
                self.camera_rot.x += rotation_speed;
            }
            ClientKeybindableActions::RotateDown => {
                self.camera_rot.x -= rotation_speed;
            }
            ClientKeybindableActions::RotateLeft => {
                self.camera_rot.y += rotation_speed;
            }
            ClientKeybindableActions::RotateRight => {
                self.camera_rot.y -= rotation_speed;
            }
            ClientKeybindableActions::RequestServerPlayerData => {
                unreachable!("Action not holdable");
            }
            ClientKeybindableActions::RequestServerChunkInfo => {
                unreachable!("Action not holdable");
            }
        }

        // clamp pitch
        self.camera_rot.x = self.camera_rot.x.clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);
    }
}
