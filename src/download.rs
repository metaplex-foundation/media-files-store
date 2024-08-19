use bytes::Bytes;
use http::StatusCode;
use thiserror::Error;

use crate::{media_type::Mime, app_metrics::{CAT_STATUS, MET_DOWNLOADS}};

/// Represents download and processing error
#[derive(Error, Debug)]
pub enum DlError {
    #[error("File is tool large: {0}")]
    FileTooLarge(u64),
    #[error("Download failed")]
    DownloadFailed,
    #[error("Not found")]
    NotFound,
    #[error("Rate limiter exceeded")]
    TooManyRequests,
    /// We probably just need to try againg later
    #[error("Server error")]
    ServerError,
    /// For now we save only images
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Processing error: {0}")]
    CorruptedAsset(String),
}

impl From<reqwest::Error> for DlError {
    fn from(_: reqwest::Error) -> Self {
        DlError::DownloadFailed
    }
}

pub async fn download(url: &str, file_max_size: u64) -> std::result::Result<(Bytes, Mime), DlError> {
    let Ok(resp) = reqwest::get(url).await else {
        metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "not_found").increment(1);
        return Err(DlError::NotFound);
    };
    if resp.status().is_client_error() {
        metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "not_found").increment(1);
        return Err(DlError::NotFound);
    }
    if resp.status().is_server_error() {
        metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "server_error").increment(1);
        return Err(DlError::ServerError);
    }
    if resp.status() == StatusCode::TOO_MANY_REQUESTS {
        metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "too_many_requests").increment(1);
        return Err(DlError::TooManyRequests);
    }
    if !resp.status().is_success() {
        metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "other_failed").increment(1);
        return Err(DlError::DownloadFailed);
    }

    let content_type = match resp.headers().get(reqwest::header::CONTENT_TYPE).map(|h|h.to_str()) {
        Some(Ok(v)) => {
            Mime::from_mime_str(v)
        },
        _ => Mime::default(),
    };

    if let Some(size) = resp.content_length() {
        if size > file_max_size {
            metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "too_large").increment(1);
            return Err(DlError::FileTooLarge(size).into());
        }
    }
    metrics::counter!(MET_DOWNLOADS, CAT_STATUS => "success").increment(1);

    let bytes = resp.bytes().await?;
    
    Ok((bytes, content_type))
}
