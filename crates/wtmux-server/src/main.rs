mod command_executor;
mod copymode;
mod pane;
mod pastebuffer;
mod renderer;
mod server;
mod session;
mod window;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("wtmux-server starting...");

    let pipe_name = wtmux_common::pipe_name();
    info!("Listening on: {}", pipe_name);

    let mut server = server::Server::new(&pipe_name)?;
    server.run().await?;

    info!("wtmux-server shutting down.");
    Ok(())
}
