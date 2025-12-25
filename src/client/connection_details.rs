use std::sync::mpsc::{Receiver, Sender};

pub enum ClientConnectionType {
    Local(LocalConnectionDetails),
    Remote(RemoteConnectionDetails),
}

pub struct LocalConnectionDetails {
    pub server_packet_receiver: Receiver<Vec<u8>>,
    pub client_packet_sender: Sender<Vec<u8>>,
}

/// Unimplemented
pub struct RemoteConnectionDetails {
    pub none: Option<()>,
}
