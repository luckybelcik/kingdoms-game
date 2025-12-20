use std::{
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{Receiver, Sender},
    },
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use nalgebra_glm::IVec3;

use crate::{
    server::prioritized_job::PrioritizedJob,
    shared::{
        chunk::Chunk,
        communication::{
            client_packet::{ClientAction, ClientPacket},
            player_data::{ConnectionType, PlayerData},
            player_id::PlayerId,
            server_packet::ServerPacket,
        },
        coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
    },
};

pub type ChunkgenJob = ChunkPos;
pub type GeneratedChunk = (ChunkPos, Chunk);

pub struct Server {
    pub chunks: HashMap<ChunkPos, ArcSwap<Chunk>>,
    pub dirty_chunks: HashSet<ChunkPos>,
    pub generating_chunks: HashSet<ChunkPos>,
    pub players: HashMap<PlayerId, PlayerData>,
    pub new_chunk_queues: HashMap<PlayerId, VecDeque<ServerPacket>>,
    pub tick: u128,
    chunkgen_job_queue: Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)>,
    generated_chunk_receiver: Receiver<GeneratedChunk>,
}

impl Server {
    pub fn new() -> Self {
        let queue: Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)> =
            Arc::new((Mutex::new(BinaryHeap::new()), Condvar::new()));
        let (generated_chunk_sender, generated_chunk_receiver) =
            std::sync::mpsc::channel::<GeneratedChunk>();

        let queue_clone = queue.clone();
        std::thread::Builder::new()
            .name("ServerChunkGenThread".to_string())
            .spawn(move || {
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
                        rayon::spawn(move || {
                            let chunk = Chunk::generate(pos.clone());
                            let _ = generated_chunk_sender_copy.send((pos, chunk));
                        });
                    }
                }
            })
            .unwrap();

        Self {
            chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            generating_chunks: HashSet::new(),
            players: HashMap::new(),
            new_chunk_queues: HashMap::new(),
            tick: 0,
            chunkgen_job_queue: queue,
            generated_chunk_receiver,
        }
    }

    pub fn run_tick_loop(&mut self) {
        const TICK_RATE: u32 = 20;
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
                self.handle_client_packet(packet);
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
        server_sender: Sender<ServerPacket>,
        client_receiver: Receiver<ClientPacket>,
    ) {
        let player_data = PlayerData {
            name: "Local".to_string(),
            position: EntityPos::new(0.0, 0.0, 0.0),
            chunk_tick_position: ChunkPos::new(0, 500, 0),
            visible_chunks: HashSet::new(),
            chunks_awaiting_generation: HashSet::new(),
            connection_type: ConnectionType::Local(server_sender, client_receiver),
            last_ping: Instant::now(),
            render_distance: 6,
        };
        self.players.insert(player_id.clone(), player_data);
        self.new_chunk_queues.insert(player_id, VecDeque::new());
    }

    pub fn handle_client_packet(&mut self, client_packet: ClientPacket) {
        let player_id = client_packet.player_id;
        match client_packet.action {
            ClientAction::Ping => {
                if let Some(player_data) = self.players.get_mut(&player_id) {
                    player_data.last_ping = Instant::now();
                }
            }
            ClientAction::Debug => {
                if let Some(player_data) = self.players.get_mut(&player_id) {
                    Self::send_packet(
                        &player_data,
                        ServerPacket::Debug(Box::new(player_data.to_sendable())),
                    );
                }
            }
        }
    }

    pub fn load_chunks(&mut self) {
        for (player_id, player_data) in self.players.iter_mut() {
            let player_chunk_pos = player_data.position.to_block_pos().to_chunk_pos();
            // If player chunk pos changed, load new chunks
            if player_chunk_pos != player_data.chunk_tick_position {
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

            player_data.chunk_tick_position = player_data.position.to_block_pos().to_chunk_pos();
        }
    }

    /// Returns true if chunk was sent, returns false if chunk was scheduled for generation
    fn load_chunk(
        chunks: &mut HashMap<ChunkPos, ArcSwap<Chunk>>,
        new_chunk_queues: &mut HashMap<PlayerId, VecDeque<ServerPacket>>,
        job_queue: &Arc<(Mutex<BinaryHeap<PrioritizedJob>>, Condvar)>,
        player_id: &PlayerId,
        player_chunk_pos: &ChunkPos,
        chunk_pos: &ChunkPos,
    ) -> bool {
        if let Some(chunk) = chunks.get(chunk_pos) {
            let chunk_arc = chunk.load_full();
            if let Some(queue) = new_chunk_queues.get_mut(player_id) {
                queue.push_back(ServerPacket::Chunk(chunk_arc.clone()));
            }
            return true;
        }

        Self::upload_chunk_for_generation(job_queue, chunk_pos, player_chunk_pos);
        return false;
    }

    fn unload_chunk(
        chunks: &mut HashMap<ChunkPos, ArcSwap<Chunk>>,
        dirty_chunks: &mut HashSet<ChunkPos>,
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
                let first_packets = queue.drain(0..std::cmp::min(5, queue.len()));

                for packet in first_packets {
                    Self::send_packet(player_data, packet);
                }
            }
        }
    }

    fn send_packet(player_data: &PlayerData, server_packet: ServerPacket) {
        match &player_data.connection_type {
            ConnectionType::Local(server_packet_sender, _) => {
                let result = server_packet_sender.send(server_packet);

                if let Err(error) = result {
                    eprintln!("Error sending packet: {}", error);
                }
            }
            ConnectionType::Remote => {
                unimplemented!("cant send remotely yet");
            }
        }
    }

    pub fn update(&mut self) {
        self.tick += 1;
    }
}

pub fn get_chunks_in_radius(player_chunk_pos: ChunkPos, radius: u8) -> HashSet<ChunkPos> {
    let mut nearby_chunks = HashSet::new();
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
