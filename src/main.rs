use configs::Settings;

mod grpc;
mod configs;
mod application;
mod asset_processing;
mod obj_storage_client;
mod media_type;
mod das_client;
mod download;
mod http_endpoints;
mod string_util;
mod image_resize;

use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let app_config = Settings::for_env("local")?;
    info!("Application config: {app_config:?}");

    application::App::start(&app_config).await;

    Ok(())
}
