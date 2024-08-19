use std::{collections::HashMap, future::ready, sync::Arc};

use axum::{
    body::Body, extract::{Path, Query, State}, http::StatusCode, response::{IntoResponse, Response}, routing::get, Router
};
use http::header::CONTENT_TYPE;
use tokio::time::Instant;
use tokio_util::io::ReaderStream;

use crate::{configs::HttpServer, image_resize::{self, ImgResizeError}, obj_storage_client::{MediaStorageClient, StoredData}};
use crate::app_metrics::setup_metrics_recorder;

const IMG_MAX_SIZE: u32 = 400;

#[derive(Clone)]
struct EndpointSharedData {
    media_storage_client: Arc<MediaStorageClient>,
}

/// Creates an HTTP server that provides asset previews to clients
pub async fn run_img_server(cfg: &HttpServer, media_storage_client: Arc<MediaStorageClient>) -> anyhow::Result<()> {
    let recorder_handle = setup_metrics_recorder();

    let state = EndpointSharedData { media_storage_client };

    let app = Router::new()
        .route("/", get(root))
        .route("/preview/:id", get(get_asset))
        .route("/metrics", get(move || { ready(recorder_handle.render())}))
        .with_state(state);

    let port = cfg.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root() -> &'static str {
    "Healthy"
}

/// Provides asset preview (image) that had been added by asset download flow.
/// 
/// Client can request asset resizing "on the fly", e.g. we store image
/// of size 400x400, but client may ask to resize it to a smaller size, like:
/// http://media-server/asset/XXXX?size=300
/// If the requested size if bigger than the size of the image in the storage,
/// then the resizing is ommited.
async fn get_asset(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    state: State<EndpointSharedData>
) -> Result<Resp, StatusCode> {
    let size_op = params.get("size")
        .and_then(|s|s.parse::<u32>().ok())
        .filter(|&s| s < IMG_MAX_SIZE);

    let start = Instant::now();

    let prview = state.media_storage_client.get_media(&id).await;
    metrics::counter!("storage_reads_total_time").increment(start.elapsed().as_millis() as u64);
    metrics::counter!("storage_reads_number").increment(1);

    let response = match prview {
        Ok(StoredData {mime, bytes: byte_stream}) => {
            match size_op {
                Some(size) => {
                    let bytes = byte_stream.collect().await.unwrap().into_bytes(); // fail on S3 error
                    match image_resize::resize_fast(&bytes, size) {
                        Ok(resized)                => Ok(Resp(mime, Body::from(resized))),
                        Err(ImgResizeError::NoResizeNeeded) => Ok(Resp(mime, Body::from(bytes))),
                        Err(_)                              => Err(StatusCode::INTERNAL_SERVER_ERROR),
                    }
                },
                None => {
                    let asset_stream = ReaderStream::new(byte_stream.into_async_read());
                    Ok(Resp(mime, Body::from_stream(asset_stream)))
                },
            }
        },
        Err(_) => Err(StatusCode::NOT_FOUND),
    };
    metrics::counter!("get_preview_requests_total_time").increment(start.elapsed().as_millis() as u64);
    metrics::counter!("get_preview_requests_number").increment(1);

    response
}

struct Resp(String, Body);

impl IntoResponse for Resp {
    fn into_response(self) -> Response {
        let Resp(mime, body) = self;
        Response::builder()
            .header(CONTENT_TYPE, mime)
            .body(body)
            .unwrap()
    }
}