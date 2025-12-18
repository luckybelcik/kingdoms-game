use std::sync::mpsc::{Receiver, Sender};

use crate::shared::communication::{client_packet::ClientPacket, server_packet::ServerPacket};

pub enum ClientConnectionType {
    Local(LocalConnectionDetails),
    Remote(RemoteConnectionDetails),
}

pub struct LocalConnectionDetails {
    pub server_packet_receiver: Receiver<ServerPacket>,
    pub client_packet_sender: Sender<ClientPacket>,
}

/// Unimplemented
pub struct RemoteConnectionDetails {
    pub none: Option<()>,
}
