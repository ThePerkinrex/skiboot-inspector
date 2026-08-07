mod app;
mod camera;
mod imu;
mod instance;
mod model;
mod resources;
mod state;
mod texture;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .try_init()
        .expect("failed to initialize tracing");
    // LogTracer::init()?;

    info!("Initialized logging");
    log::info!("Hello from log");

    app::run()?;
    // imu::run().await?;

    Ok(())
}
