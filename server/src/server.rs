use std::{
    collections::{BinaryHeap, HashMap, VecDeque},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{Receiver, Sender},
    },
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use engine_assets::{
    block_registry::{self, BlockRegistry},
    projects::Project,
};
use engine_core::{chunk_pos::ChunkPos, entity_pos::EntityPos};
use engine_net::{
    client_actions::PlayerActions,
    client_packet::{ClientAction, ClientPacket},
    player_data::{ClientPlayerData, ConnectionType, PlayerData, PlayerPermissions},
    player_id::PlayerId,
    server_packet::{DebugChunkData, DenialReason, ServerPacket},
};
use engine_world::chunk::Chunk;
use lasso::ThreadedRodeo;
use nalgebra_glm::{IVec3, distance};
use rustc_hash::{FxHashMap, FxHashSet};
use shared_utils::raycast::cast_ray;

use crate::constants::{MAX_ACCEPTABLE_POSITION_DELTA, MAX_NEW_CHUNK_COUNT, MOVE_SPEED, TICK_RATE};
use crate::prioritized_job::PrioritizedJob;

pub type GeneratedChunk = (ChunkPos, Chunk);

pub struct Server {
    pub block_registry: Arc<BlockRegistry>,
    pub chunks: FxHashMap<ChunkPos, ArcSwap<Chunk>>,
    pub dirty_chunks: FxHashSet<ChunkPos>,
    pub generating_chunks: FxHashSet<ChunkPos>,
    pub players: HashMap<PlayerId, PlayerData>,
    pub new_chunk_queues: HashMap<PlayerId, VecDeque<ServerPacket>>,
    pub tick: u128,
    chunkgen_job_queue: Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)>,
    generated_chunk_receiver: Receiver<GeneratedChunk>,
    interner: Arc<ThreadedRodeo>,
}

impl Server {
    pub fn new() -> Self {
        let queue: Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)> =
            Arc::new((Mutex::new(BinaryHeap::new()), Condvar::new()));
        let (generated_chunk_sender, generated_chunk_receiver) =
            std::sync::mpsc::channel::<GeneratedChunk>();

        let mut projects_to_load = Vec::new();
        let all_projects = Project::find_all();
        for proj in all_projects {
            projects_to_load.push(proj);
        }

        let interner = Arc::new(ThreadedRodeo::new());

        let block_registry_context = BlockRegistry::init(projects_to_load, false, &interner);
        let block_registry = block_registry_context.block_registry;
        let block_registry_arc = Arc::new(block_registry);
        let block_registry_arc_clone = block_registry_arc.clone();

        let queue_clone = queue.clone();
        std::thread::Builder::new()
            .name("ServerChunkGenThread".to_string())
            .spawn(move || {
                let block_registry_arc_clone_2 = block_registry_arc_clone;
                loop {
                    let (lock, cvar) = &*queue_clone;
                    let mut heap = lock.lock().unwrap();

                    while heap.is_empty() {
                        heap = cvar.wait(heap).unwrap();
                    }

                    if let Some(job) = heap.pop() {
                        drop(heap);

                        let pos = job.pos;
                        let generated_chunk_sender_copy = generated_chunk_sender.clone();
                        let block_registry_arc_clone_3 = block_registry_arc_clone_2.clone();
                        rayon::spawn(move || {
                            let chunk = Chunk::generate(pos.clone(), block_registry_arc_clone_3);
                            let _ = generated_chunk_sender_copy.send((pos, chunk));
                        });
                    }
                }
            })
            .unwrap();

        Self {
            block_registry: block_registry_arc,
            chunks: FxHashMap::default(),
            dirty_chunks: FxHashSet::default(),
            generating_chunks: FxHashSet::default(),
            players: HashMap::new(),
            new_chunk_queues: HashMap::new(),
            tick: 0,
            chunkgen_job_queue: queue,
            generated_chunk_receiver,
            interner,
        }
    }

    pub fn run_tick_loop(&mut self) {
        let tick_duration: Duration = Duration::from_secs_f64(1.0 / TICK_RATE as f64);

        loop {
            let frame_start = Instant::now();

            self.update();

            let mut packets = Vec::new();

            for player in self.players.values() {
                match &player.connection_type {
                    ConnectionType::Local(_sender, receiver) => {
                        while let Ok(packet) = receiver.try_recv() {
                            packets.push(packet);
                        }
                    }
                    ConnectionType::Remote => {
                        unimplemented!("Remoted players not implemented yet")
                    }
                }
            }

            for packet in packets {
                if let Ok(client_packet) = bincode::deserialize(&packet) {
                    self.handle_client_packet(client_packet);
                }
            }

            self.receive_chunk_from_generation();
            self.load_chunks();
            self.send_chunk_packets();

            let elapsed = frame_start.elapsed();

            if elapsed < tick_duration {
                std::thread::sleep(tick_duration - elapsed);
            } else {
                eprintln!("Server tick took too long: {:?}", elapsed);
            }
        }
    }

    pub fn add_local_player(
        &mut self,
        player_id: PlayerId,
        server_sender: Sender<Vec<u8>>,
        client_receiver: Receiver<Vec<u8>>,
    ) {
        let player_data = PlayerData {
            player_permissions: PlayerPermissions::Admin,
            name: "Local".to_string(),
            position: EntityPos::new(0.0, 0.0, 0.0),
            chunk_tick_position: ChunkPos::new(0, 500, 0),
            visible_chunks: FxHashSet::default(),
            chunks_awaiting_generation: FxHashSet::default(),
            connection_type: ConnectionType::Local(server_sender, client_receiver),
            last_ping: Instant::now(),
            render_distance: 6,
            selected_block: 1,
        };
        Self::send_packet(
            &player_data,
            ServerPacket::PlayerData(player_data.to_client_data()),
        );
        self.players.insert(player_id.clone(), player_data);
        self.new_chunk_queues.insert(player_id, VecDeque::new());
    }

    pub fn handle_client_packet(&mut self, client_packet: ClientPacket) {
        let player_id = client_packet.player_id;
        if let Some(player_data) = self.players.get_mut(&player_id) {
            match client_packet.action {
                ClientAction::Ping => {
                    player_data.last_ping = Instant::now();
                }
                ClientAction::RequestPlayerData => {
                    Self::send_packet(
                        &player_data,
                        ServerPacket::PlayerData(player_data.to_client_data()),
                    );
                }
                ClientAction::PlayerAction(action) => match action {
                    PlayerActions::BreakBlock(rot, pos) => {
                        let distance = distance(&pos, &player_data.position);
                        if distance < MAX_ACCEPTABLE_POSITION_DELTA {
                            if let Some(raycast_result) = cast_ray(pos, rot, &self.chunks, 64) {
                                if let Some(chunk) = self.chunks.get_mut(&raycast_result.hit.0) {
                                    let mut new_client_chunk = (*(chunk.load_full())).clone();
                                    new_client_chunk.set_block(
                                        raycast_result.hit.1,
                                        0,
                                        &mut self.dirty_chunks,
                                    );
                                    chunk.store(Arc::new(new_client_chunk));
                                }
                            }
                        }
                    }
                    PlayerActions::PlaceBlock(rot, pos) => {
                        let distance = distance(&pos, &player_data.position);
                        if distance < MAX_ACCEPTABLE_POSITION_DELTA {
                            if let Some(raycast_result) = cast_ray(pos, rot, &self.chunks, 64) {
                                if let Some(chunk) = self.chunks.get_mut(&raycast_result.previous.0)
                                {
                                    let mut new_client_chunk = (*(chunk.load_full())).clone();
                                    new_client_chunk.set_block(
                                        raycast_result.previous.1,
                                        player_data.selected_block,
                                        &mut self.dirty_chunks,
                                    );
                                    chunk.store(Arc::new(new_client_chunk));
                                }
                            }
                        }
                    }
                    PlayerActions::ChangeSelectedBlock(block_id) => {
                        if let Some(id) = self.block_registry.get_block(&block_id) {
                            player_data.selected_block = *id;
                        }
                    }
                    PlayerActions::MoveForwards(rot) => {
                        let (sin_y, cos_y) = rot.y.sin_cos();
                        player_data.position.x += cos_y * MOVE_SPEED;
                        player_data.position.z += sin_y * MOVE_SPEED;
                    }
                    PlayerActions::MoveBackwards(rot) => {
                        let (sin_y, cos_y) = rot.y.sin_cos();
                        player_data.position.x -= cos_y * MOVE_SPEED;
                        player_data.position.z -= sin_y * MOVE_SPEED;
                    }
                    PlayerActions::MoveLeft(rot) => {
                        let (sin_y, cos_y) = rot.y.sin_cos();
                        player_data.position.x -= sin_y * MOVE_SPEED;
                        player_data.position.z += cos_y * MOVE_SPEED;
                    }
                    PlayerActions::MoveRight(rot) => {
                        let (sin_y, cos_y) = rot.y.sin_cos();
                        player_data.position.x += sin_y * MOVE_SPEED;
                        player_data.position.z -= cos_y * MOVE_SPEED;
                    }
                    PlayerActions::MoveUp => {
                        player_data.position.y += MOVE_SPEED;
                    }
                    PlayerActions::MoveDown => {
                        player_data.position.y -= MOVE_SPEED;
                    }
                    PlayerActions::ScrollHotbarRight => {
                        player_data.selected_block += 1;
                    }
                    PlayerActions::ScrollHotbarLeft => {
                        player_data.selected_block -= 1;
                    }
                },
                ClientAction::DebugPlayer => {
                    Self::send_packet_if_permitted(
                        &player_data,
                        ServerPacket::DebugPlayer(Box::new(player_data.to_sendable())),
                        PlayerPermissions::Helper,
                    );
                }
                ClientAction::DebugChunks => {
                    Self::send_packet_if_permitted(
                        &player_data,
                        ServerPacket::DebugChunk(Box::new(DebugChunkData {
                            chunk_count: self.chunks.len() as u32,
                            dirty_chunks: self.dirty_chunks.len() as u32,
                            generating_chunks: self.generating_chunks.len() as u32,
                        })),
                        PlayerPermissions::Helper,
                    );
                }
                ClientAction::DebugCheckSync(received_data) => {
                    let stored_data = player_data.to_client_data();
                    ClientPlayerData::log_desync(&stored_data, &received_data);
                }
            }
        }
    }

    pub fn load_chunks(&mut self) {
        for (player_id, player_data) in self.players.iter_mut() {
            let player_chunk_pos = player_data.position.to_block_pos().to_chunk_pos();

            let load_new_chunks = self.tick.is_multiple_of(19) // only update per 20 ticks
                && player_chunk_pos
                    != player_data.chunk_tick_position;
            // If player chunk pos changed, load new chunks
            if load_new_chunks {
                let current_nearby_chunks =
                    get_chunks_in_radius(player_chunk_pos, player_data.render_distance);
                let old_nearby_chunks = get_chunks_in_radius(
                    player_data.chunk_tick_position,
                    player_data.render_distance,
                );

                let to_load: Vec<ChunkPos> = current_nearby_chunks
                    .difference(&old_nearby_chunks)
                    .cloned()
                    .collect();
                let to_unload: Vec<ChunkPos> = old_nearby_chunks
                    .difference(&current_nearby_chunks)
                    .cloned()
                    .collect();

                for chunk_pos in to_load.iter().as_ref() {
                    let result = Self::load_chunk(
                        &mut self.chunks,
                        &mut self.new_chunk_queues,
                        &self.chunkgen_job_queue,
                        player_id,
                        &player_chunk_pos,
                        chunk_pos,
                    );

                    if result == false {
                        player_data
                            .chunks_awaiting_generation
                            .insert(chunk_pos.clone());
                    }
                }

                for chunk_pos in to_unload.iter().as_ref() {
                    Self::unload_chunk(&mut self.chunks, &mut self.dirty_chunks, chunk_pos);
                }

                player_data.visible_chunks = current_nearby_chunks;
            }

            let mut pos_to_remove = Vec::new();

            // Check if old chunks got generated and load them if they did
            for awaited_chunk_pos in player_data.chunks_awaiting_generation.iter() {
                if self.chunks.contains_key(awaited_chunk_pos) {
                    let result = Self::load_chunk(
                        &mut self.chunks,
                        &mut self.new_chunk_queues,
                        &self.chunkgen_job_queue,
                        player_id,
                        &player_chunk_pos,
                        awaited_chunk_pos,
                    );

                    if result {
                        pos_to_remove.push(awaited_chunk_pos.clone());
                    }
                }
            }

            for pos in pos_to_remove {
                let _ = player_data.chunks_awaiting_generation.remove(&pos);
            }

            if self.tick.is_multiple_of(19) {
                player_data.chunk_tick_position =
                    player_data.position.to_block_pos().to_chunk_pos();
            }
        }
    }

    /// Returns true if chunk was sent, returns false if chunk was scheduled for generation
    fn load_chunk(
        chunks: &mut FxHashMap<ChunkPos, ArcSwap<Chunk>>,
        new_chunk_queues: &mut HashMap<PlayerId, VecDeque<ServerPacket>>,
        job_queue: &Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)>,
        player_id: &PlayerId,
        player_chunk_pos: &ChunkPos,
        chunk_pos: &ChunkPos,
    ) -> bool {
        if let Some(chunk) = chunks.get(chunk_pos) {
            let chunk_arc = chunk.load_full();
            if let Some(queue) = new_chunk_queues.get_mut(player_id) {
                queue.push_back(ServerPacket::Chunk(Box::new((*chunk_arc).clone())));
            }
            return true;
        }

        Self::upload_chunk_for_generation(job_queue, chunk_pos, player_chunk_pos);
        return false;
    }

    fn unload_chunk(
        chunks: &mut FxHashMap<ChunkPos, ArcSwap<Chunk>>,
        dirty_chunks: &mut FxHashSet<ChunkPos>,
        chunk_pos: &ChunkPos,
    ) {
        if chunks.contains_key(chunk_pos) {
            chunks.remove(chunk_pos);
        }

        if dirty_chunks.contains(chunk_pos) {
            dirty_chunks.remove(chunk_pos);
        }
    }

    fn upload_chunk_for_generation(
        job_queue: &Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)>,
        chunk_pos: &ChunkPos,
        player_pos: &ChunkPos,
    ) {
        let (lock, cvar) = &**job_queue;
        let mut heap = lock.lock().unwrap();

        let dist = (chunk_pos.x - player_pos.x).abs()
            + (chunk_pos.y - player_pos.y).abs()
            + (chunk_pos.z - player_pos.z).abs();

        heap.push(PrioritizedJob {
            priority: dist,
            pos: chunk_pos.clone(),
        });
        cvar.notify_one();
    }

    pub fn receive_chunk_from_generation(&mut self) {
        while let Ok((pos, chunk)) = self.generated_chunk_receiver.try_recv() {
            self.chunks.insert(pos, ArcSwap::new(Arc::new(chunk)));
        }
    }

    pub fn send_chunk_packets(&mut self) {
        for (player_id, queue) in self.new_chunk_queues.iter_mut() {
            if let Some(player_data) = self.players.get(player_id) {
                let first_packets = queue.drain(0..std::cmp::min(MAX_NEW_CHUNK_COUNT, queue.len()));

                if let ConnectionType::Local(sender, _) = &player_data.connection_type {
                    for packet in first_packets {
                        let sender_clone = sender.clone();

                        rayon::spawn(move || {
                            let bytes = bincode::serialize(&packet).unwrap();
                            let result = sender_clone.send(bytes);

                            if let Err(error) = result {
                                eprintln!("Error sending packet: {}", error);
                            }
                        });
                    }
                } else {
                    unimplemented!("we dont do remote connections yet, so sowwy!");
                }
            }
        }
    }

    fn send_packet(player_data: &PlayerData, server_packet: ServerPacket) {
        match &player_data.connection_type {
            ConnectionType::Local(server_packet_sender, _) => {
                let bytes = bincode::serialize(&server_packet).unwrap();
                let result = server_packet_sender.send(bytes);

                if let Err(error) = result {
                    eprintln!("Error sending packet: {}", error);
                }
            }
            ConnectionType::Remote => {
                unimplemented!("cant send remotely yet");
            }
        }
    }

    fn send_packet_if_permitted(
        player_data: &PlayerData,
        server_packet: ServerPacket,
        minimum_permission: PlayerPermissions,
    ) {
        if player_data.player_permissions >= minimum_permission {
            Self::send_packet(player_data, server_packet);
        } else {
            Self::send_packet(
                player_data,
                ServerPacket::Denial(DenialReason::InsufficientPermissions),
            );
        }
    }

    pub fn update(&mut self) {
        self.tick += 1;
    }
}

pub fn get_chunks_in_radius(player_chunk_pos: ChunkPos, radius: u8) -> FxHashSet<ChunkPos> {
    let mut nearby_chunks = FxHashSet::default();
    let radius = radius as i32;
    let radius_sq = (radius * radius) as f32;

    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                let offset = IVec3::new(x, y, z);

                if (x * x + y * y + z * z) as f32 <= radius_sq {
                    nearby_chunks.insert(ChunkPos::new_from_vec(*player_chunk_pos + offset));
                }
            }
        }
    }

    nearby_chunks
}
