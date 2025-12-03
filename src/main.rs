#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::client::rendering::app::App;

pub mod client;
pub mod shared;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        #[cfg(target_os = "linux")]
        let event_loop = winit::event_loop::EventLoop::builder().with_x11().build()?;

        #[cfg(not(target_os = "linux"))]
        let event_loop = winit::event_loop::EventLoop::builder().build()?;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let mut app = App::default();
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
