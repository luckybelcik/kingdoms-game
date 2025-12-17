#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    client::rendering::app::App,
    server::server::Server,
    shared::communication::{
        client_packet::ClientPacket, player_data::ConnectionType, player_id::PlayerId,
        server_packet::ServerPacket,
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

        std::thread::spawn(move || {
            let mut server = Server::new();

            server.add_local_player(player_id_clone, server_sender, client_receiver);

            loop {
                let mut packets = Vec::new();

                for player in server.players.values() {
                    match &player.connection_type {
                        ConnectionType::Local(_sender, receiver) => {
                            for packet in receiver {
                                packets.push(packet);
                            }
                        }
                        ConnectionType::Remote => {
                            unimplemented!("Remoted players not implemented yet")
                        }
                    }
                }

                for packet in packets {
                    server.handle_client_packet(packet);
                }
            }
        });

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let mut app = App::new(player_id, server_receiver, client_sender);
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
