use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body, extract::{Path, Query, State}, http::StatusCode, response::{IntoResponse, Response}, routing::get, Router
};
use http::header::CONTENT_TYPE;
use tokio_util::io::ReaderStream;

use crate::{configs::HttpServer, image_resize::{self, ImgResizeError}, obj_storage_client::{MediaStorageClient, StoredData}};

const IMG_MAX_SIZE: u32 = 400;

#[derive(Clone)]
struct EndpointSharedData {
    media_storage_client: Arc<MediaStorageClient>,
}

/// Creates an HTTP server that provides asset previews to clients
pub async fn run_img_server(http_config: &HttpServer, media_storage_client: Arc<MediaStorageClient>) -> anyhow::Result<()> {

    let state = EndpointSharedData { media_storage_client };

    let app = Router::new()
        .route("/", get(root))
        .route("/asset/:id", get(get_asset))
        .with_state(state);

    let port = http_config.port;
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
) -> impl IntoResponse {
    let size_op = params.get("size")
        .and_then(|s|s.parse::<u32>().ok())
        .filter(|&s| s < IMG_MAX_SIZE);

    match state.media_storage_client.get_media(&id).await {
        Ok(StoredData {mime, bytes: byte_stream}) => {
            match size_op {
                Some(size) => {
                    let bytes = byte_stream.collect().await.unwrap().into_bytes(); // fail on S3 error
                    match image_resize::resize_fast(&bytes, size) {
                        Ok(resized) =>
                            Response::builder()
                                .header(CONTENT_TYPE, mime)
                                .body(Body::from(resized))
                                .unwrap(),
                        Err(ImgResizeError::NoResizeNeeded) =>
                            Response::builder()
                                .header(CONTENT_TYPE, mime)
                                .body(Body::from(bytes))
                                .unwrap(),
                        Err(_) =>
                            Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::empty())
                                .unwrap()
                    }
                },
                None => {
                    let asset_stream = ReaderStream::new(byte_stream.into_async_read());
                    Response::builder()
                        .header(CONTENT_TYPE, mime)
                        .body(Body::from_stream(asset_stream))
                        .unwrap()
                },
            }
        },
        Err(_) => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    }
}
