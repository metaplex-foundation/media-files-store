use bytes::Bytes;
use http::StatusCode;
use thiserror::Error;

use crate::media_type::Mime;

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
        return Err(DlError::NotFound);
    };
    if resp.status().is_client_error() {
        return Err(DlError::NotFound);
    }
    if resp.status().is_server_error() {
        return Err(DlError::ServerError);
    }
    if resp.status() == StatusCode::TOO_MANY_REQUESTS {
        return Err(DlError::TooManyRequests);
    }
    if !resp.status().is_success() {
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
            return Err(DlError::FileTooLarge(size).into());
        }
    }

    let bytes = resp.bytes().await?;
    
    Ok((bytes, content_type))
}
