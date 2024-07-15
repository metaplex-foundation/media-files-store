use std::sync::Arc;

use crate::{configs::Settings, das_client::UtilityChainClient, http_endpoints, asset_processing, obj_storage_client::MediaStorageClient};

pub struct App {
}

impl App {
    /// This is the main assembly point for the media-service application.
    /// In starts the URL fetcher that continuously queries DAS node for new URLs to download,
    /// and HTTP server for providing assets preview images.
    pub async fn start(app_cfg: &Settings) {
        let media_storag_client = Arc::new(MediaStorageClient::new(&app_cfg.obj_storage).await);

        if app_cfg.das.enabled {
            // Rollup NFTs downloader
            let das_client = UtilityChainClient { das_url: app_cfg.das.grpc_address.clone() };
            asset_processing::start_downloading_pipeline(Arc::new(das_client), media_storag_client.clone(), &app_cfg.das)
                .await;
        }
        
        if app_cfg.http_server.enabled {
            // Provides downloaded NFT assets via HTTP
            http_endpoints::run_img_server(&app_cfg.http_server, media_storag_client.clone())
                .await.unwrap();
        }

    }
}