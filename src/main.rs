#![feature(int_roundings)]

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    client::{
        app::app::App,
        connection_details::{ClientConnectionType, LocalConnectionDetails},
    },
    server::server::Server,
    shared::communication::{
        client_packet::ClientPacket, player_id::PlayerId, server_packet::ServerPacket,
    },
};

pub mod client;
pub mod server;
pub mod shared;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        #[cfg(target_os = "linux")]
        let event_loop = winit::event_loop::EventLoop::builder().with_x11().build()?;

        #[cfg(not(target_os = "linux"))]
        let event_loop = winit::event_loop::EventLoop::builder().build()?;

        let (server_sender, server_receiver) = std::sync::mpsc::channel::<ServerPacket>();
        let (client_sender, client_receiver) = std::sync::mpsc::channel::<ClientPacket>();

        let player_id = PlayerId::new();
        let player_id_clone = player_id.clone();

        std::thread::Builder::new()
            .name("MainServerThread".to_string())
            .spawn(move || {
                let mut server = Server::new();

                server.add_local_player(player_id_clone, server_sender, client_receiver);

                server.run_tick_loop();
            })
            .unwrap();

        println!("Server set up successfully! Jump jump jump!");

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let local_connection_details = LocalConnectionDetails {
            server_packet_receiver: server_receiver,
            client_packet_sender: client_sender,
        };
        let connection_type = ClientConnectionType::Local(local_connection_details);
        let mut app = App::new(player_id, connection_type);
        println!("App created successfully!");
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
