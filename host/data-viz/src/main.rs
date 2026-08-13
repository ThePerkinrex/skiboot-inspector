mod app;
mod imu;

use tracing::info;
use winit::event_loop::EventLoop;

use crate::app::App;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = rt.handle().clone();
    tracing_subscriber::fmt()
        .try_init()
        .expect("failed to initialize tracing");
    // LogTracer::init()?;

    info!("Initialized logging");
    log::info!("Hello from log");

    let event_loop = EventLoop::with_user_event().build()?;

    let mut app = App::new(handle, event_loop.create_proxy());
    event_loop.run_app(&mut app)?;
    // imu::run().await?;

    Ok(())
}
