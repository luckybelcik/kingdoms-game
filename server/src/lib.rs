use std::sync::mpsc::{Receiver, Sender};

use engine_net::player_id::PlayerId;

use crate::server::Server;
mod constants;
mod prioritized_job;
mod server;

pub fn spawn_integrated_server(player_id: PlayerId) -> (Sender<Vec<u8>>, Receiver<Vec<u8>>) {
    let (server_sender, server_receiver) = std::sync::mpsc::channel();
    let (client_sender, client_receiver) = std::sync::mpsc::channel();

    let player_id_clone = player_id.clone();
    std::thread::Builder::new()
        .name("MainServerThread".to_string())
        .spawn(move || {
            let mut server = Server::new();
            server.add_local_player(player_id_clone, server_sender, client_receiver);
            server.run_tick_loop();
        })
        .expect("Failed to spawn server thread");

    (client_sender, server_receiver)
}
