use engine_net::player_id::PlayerId;
use winit::error::EventLoopError;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    app::{app::App, crash_handler},
    connection_details::{ClientConnectionType, LocalConnectionDetails},
};

pub mod app;
pub mod client;
pub mod connection_details;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    crash_handler::init();
    engine_core::paths::init_data_path()?;

    let player_id = PlayerId::new();
    let (tx, rx) = engine_server::spawn_integrated_server(player_id.clone());

    let event_loop = setup_event_loop().unwrap();
    let connection = ClientConnectionType::Local(LocalConnectionDetails {
        server_packet_receiver: rx,
        client_packet_sender: tx,
    });

    let mut app = App::new(player_id, connection);
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn setup_event_loop() -> Result<winit::event_loop::EventLoop<()>, EventLoopError> {
    #[cfg(target_os = "linux")]
    return winit::event_loop::EventLoop::builder().with_x11().build();

    #[cfg(not(target_os = "linux"))]
    return winit::event_loop::EventLoop::builder().build();
}
