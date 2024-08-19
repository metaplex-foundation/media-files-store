use std::sync::Arc;

use tokio::time::Instant;

use crate::{
    configs::{AssetProcessorCfg, DasCfg},
    das_client::{DasClient, DlOutcome, UrlDlResult},
    download::download,
    image_resize::{self, ImgResizeError},
    media_type::AssetClass,
    obj_storage_client::MediaStorageClient,
    string_util::keccak256_hash_bs58str
};

const SEND_BACK_BUFFER_SIZE: usize = 100;

pub enum Task {
    /// ID, URL
    Download { url: String },
    /// We use this to decrease the number of download workers in runtime if needed
    Finish
}

pub struct TaskResp(UrlDlResult);

/// The whole processing schema looks as following:
/// ```no-syntax
/// +--------------------------------+
/// |          Utility-chain         |
/// +--------------------------------+
///   ||                           /\
///  \||/                         /||\ 
///   \/     --> worker --         ||
///         /              \       
/// poller ----> worker ----> result_sender
///         \              /
///          --> worker --
/// ```
/// No need for graceful shutdown because, downloaded assets are persited in
/// a idempotent way, i.e. at least once semantics is perfectly fine for us.
pub async fn start_downloading_pipeline(
    das_client: Arc<dyn DasClient + Send + Sync + 'static>,
    media_storage: Arc<MediaStorageClient>,
    das_cfg: &DasCfg,
    asset_cfg: &AssetProcessorCfg,
) {
    let tasks_queue_size = das_cfg.number_of_workers * das_cfg.fetch_batch_size as usize;
    let (resp_sender, resp_recv) = tokio::sync::mpsc::channel::<TaskResp>(tasks_queue_size);
    let (task_sender, task_recv) = async_channel::bounded::<Task>(tasks_queue_size);

    for _ in 0 .. das_cfg.number_of_workers {
        make_worker(task_recv.clone(), resp_sender.clone(), media_storage.clone(), asset_cfg.clone()).await;    
    }

    make_poller(das_client.clone(), task_sender, das_cfg.fetch_batch_size).await;
    make_results_sender(das_client.clone(),resp_recv).await;
}

async fn make_poller(
    das_client: Arc<dyn DasClient + Send + Sync + 'static>,
    task_sender: async_channel::Sender<Task>,
    poll_batch_size: u32,
) {
    tokio::spawn(async move {
        loop {
            let to_process = das_client.fetch_assets_for_downloading(poll_batch_size).await;
            for asset in to_process {
                task_sender.send(Task::Download { url: asset })
                    .await.unwrap();
            }
        }
    });
}

async fn make_results_sender(das_client: Arc<dyn DasClient + Send + Sync + 'static>, mut resp_recv: tokio::sync::mpsc::Receiver<TaskResp>) {
    tokio::spawn(async move {
        let mut buffer: Vec<UrlDlResult> = Vec::new(); // NFT Id -> mime type
        let mut start = Instant::now();
        loop {
            match resp_recv.recv().await {
                Some(TaskResp(asset_download_result)) => {
                    buffer.push(asset_download_result);

                    if buffer.len() >= SEND_BACK_BUFFER_SIZE {
                        let latency = start.elapsed().as_secs_f64();
                        metrics::gauge!("flow_rate").set(latency / SEND_BACK_BUFFER_SIZE as f64);
                        start = Instant::now();

                        das_client.notify_finished(buffer).await;
                        buffer = Vec::new();
                    }
                },
                None => break,
            }
        }
        if !buffer.is_empty() {
            das_client.notify_finished(buffer).await;
        }
    });
}

async fn make_worker(
    requests: async_channel::Receiver<Task>,
    responses: tokio::sync::mpsc::Sender<TaskResp>,
    media_storage: Arc<MediaStorageClient>,
    asset_cfg: AssetProcessorCfg,
) {
    tokio::spawn(async move {
        metrics::gauge!("workers_count").increment(1);
        loop {
            match requests.recv().await {
                Ok(msg) => {
                    match msg {
                        Task::Download { url} => {
                            let asset_download_result = process_url(url, &media_storage, &asset_cfg).await;
                            match responses.send(TaskResp(asset_download_result)).await {
                                Ok(_) => (),
                                Err(_) => break,
                            }
                        },
                        Task::Finish => break,
                    }
                },
                Err(_) => break,
            }
        }
        metrics::gauge!("workers_count").decrement(1);
    });

    async fn process_url(url: String, media_storage: &MediaStorageClient, asset_cfg: &AssetProcessorCfg) -> UrlDlResult {
        let start = Instant::now();

        let id = keccak256_hash_bs58str(&url);
        let asset_download_result = match download(&url, asset_cfg.file_max_size_bytes).await {
            Ok((bytes, mime)) => {
                if mime.class == AssetClass::Image {
                    match image_resize::resize_fast(&bytes, asset_cfg.resize_to) {
                        Ok(resized) => {
                            media_storage.save_media(&id, resized.into(),mime.str()).await.unwrap();
                            UrlDlResult { url, outcome: DlOutcome::success(mime.str(), asset_cfg.resize_to) }
                        },
                        Err(ImgResizeError::NoResizeNeeded) => {
                            media_storage.save_media(&id, bytes.into(),mime.str()).await.unwrap();
                            UrlDlResult { url, outcome: DlOutcome::success(mime.str(), asset_cfg.resize_to) }       
                        },
                        Err(err) =>
                            UrlDlResult { url, outcome: DlOutcome::corrupted_asset(err.to_string()) }
                    }
                } else {
                    UrlDlResult { url, outcome: DlOutcome::unsupported_format(mime.str()) }
                }
            } ,
            Err(err) => {
                UrlDlResult { url, outcome: err.into() }
            },
        };

        metrics::histogram!("asset_processing").record(start.elapsed().as_secs_f64());

        asset_download_result
    }
}
