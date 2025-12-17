use std::{
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};

use crate::shared::communication::{client_packet::ClientPacket, server_packet::ServerPacket};

pub struct PlayerData {
    pub name: String,
    pub position: nalgebra_glm::Vec3,
    pub chunk_tick_position: nalgebra_glm::Vec3,
    pub connection_type: ConnectionType,
    pub last_ping: Instant,
    pub render_distance: u8,
}

pub enum ConnectionType {
    Local(Sender<ServerPacket>, Receiver<ClientPacket>),
    Remote,
}
