use tracing_subscriber::util::SubscriberInitExt;

use crate::server::MultivmServer;

mod server;
mod utils;

static mut MULTIVM_SERVER: Option<MultivmServer> = None;

pub fn install_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap();

    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init()
        .unwrap();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_tracing();

    unsafe {
        MULTIVM_SERVER = Some(MultivmServer::new());
        MULTIVM_SERVER.as_mut().unwrap().start().await
    }
}
