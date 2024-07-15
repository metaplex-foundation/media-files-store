use async_trait::async_trait;

use crate::{grpc::asseturls::{asset_url_service_client::AssetUrlServiceClient, url_download_details::DlResult, DownloadError, DownloadResultsRequest, DownloadSuccess, GetAssetUrlsRequest, UrlDownloadDetails}, download::DlError};

/// Interface for DAS node (utility-chain) client
#[async_trait]
pub trait DasClient {

    /// Requests batch of asset URLs to download and save as previews.
    /// ## Arguments:
    /// * `amount` - maximum number of URLs to fetch
    async fn fetch_assets_for_downloading(&self, amount: u32) -> Vec<String>;

    /// Send to DAS node information about processed URLs
    /// ## Arguments:
    /// * `asset_result` - collection of asset download and processing results
    async fn notify_finished(&self, asset_result: Vec<UrlDlResult>);
}

pub struct UtilityChainClient {
    pub das_url: String,
}

#[async_trait]
impl DasClient for UtilityChainClient {
    async fn fetch_assets_for_downloading(&self, amount: u32) -> Vec<String> {
        let url = self.das_url.clone();
        let Ok(mut client) = AssetUrlServiceClient::connect(url).await else {
            return Vec::new();
        };
        let request = tonic::Request::new(GetAssetUrlsRequest { count: amount});
    
        match client.get_asset_urls_to_download(request).await {
            Ok(resp) => resp.into_inner().urls,
            Err(err) => {
                eprintln!("{err:?}");
                Vec::new()
            },
        }
    }

    async fn notify_finished(&self, asset_result: Vec<UrlDlResult>) {
        let results: Vec<UrlDownloadDetails> = asset_result.into_iter()
            .map(|UrlDlResult {url, outcome }|
                UrlDownloadDetails { url, dl_result: Some(outcome.into()) }
            )
            .collect::<Vec<_>>();

        let url = self.das_url.clone();
        let Ok(mut client) = AssetUrlServiceClient::connect(url).await else {
            return ();
        };
        let request = tonic::Request::new(DownloadResultsRequest { results });
        match client.submit_download_result(request).await {
            Ok(_) => (),
            Err(_) => (),
        }
    }
}

/// URL processing result
pub struct  UrlDlResult {
    /// Asset URL that has been processed
    pub url: String,
    /// Prcessing result which is either successful download or download error
    pub outcome: DlOutcome,
}

pub enum DlOutcome {
    Success { mime: String, size: u32 },
    Fail { err: crate::download::DlError }
}

impl DlOutcome {
    pub fn success(mime: &str, size: u32) -> DlOutcome {
        DlOutcome::Success { mime: mime.to_string(), size }
    }
    pub fn unsupported_format(mime: &str) -> DlOutcome {
        DlOutcome::Fail { err: DlError::UnsupportedFormat(mime.to_string()) }
    }
}

impl From<DlOutcome> for DlResult {
    fn from(value: DlOutcome) -> Self {
        match value {
            DlOutcome::Success { mime, size } =>
                DlResult::Success(DownloadSuccess { mime, size }),
            DlOutcome::Fail { err } =>
                DlResult::Fail(<DlError as Into<DownloadError>>::into(err) as i32),
        }
    }
}

impl From<DlError> for DownloadError {
    fn from(value: DlError) -> Self {
        use DlError as E;
        match value {
            E::FileTooLarge(_) => DownloadError::TooLarge,
            E::DownloadFailed | E::NotFound => DownloadError::NotFound,
            E::ServerError => DownloadError::ServerError,
            E::UnsupportedFormat(_) => DownloadError::NotSupportedFormat,
        }
    }
}

impl From<DlError> for DlOutcome {
    fn from(value: DlError) -> Self {
        DlOutcome::Fail { err: value }
    }
}
