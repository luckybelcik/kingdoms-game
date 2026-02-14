use crate::server::Server;

mod constants;
mod prioritized_job;
mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    engine_core::paths::init_data_path()?;

    println!("Starting dedicated server...");
    let mut server = Server::new();
    // Instead of channels, here you would set up TCP/UDP listeners
    server.run_tick_loop();
    Ok(())
}
