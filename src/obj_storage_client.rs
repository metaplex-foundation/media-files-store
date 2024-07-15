
use aws_config::Region;
use aws_sdk_s3::{config::Credentials, primitives::ByteStream};

use crate::configs::ObjStorage;


/// Wrapper for S3 client that provides convenent API for storing asset previews
pub struct MediaStorageClient {
    s3_client: aws_sdk_s3::Client,
    media_bucket: String,
}

pub struct StoredData {
    pub bytes: ByteStream,
    pub mime: String,
}

impl MediaStorageClient {
    pub async fn new(cfg: &ObjStorage) -> MediaStorageClient {
        let media_bucket = cfg.bucket_for_media.clone();

        let mut config_loader = aws_config::from_env();
        if let Some(reg) = &cfg.region {
            config_loader = config_loader.region(Region::new(reg.clone()));
        }
        if let Some(endpoint) = &cfg.endpoint {
            config_loader = config_loader.endpoint_url(endpoint.clone());
        }
        if let (Some(key), Some(passwd)) = (&cfg.access_key_id, &cfg.secret_access_key) {
            let creds = Credentials::new(key, passwd, cfg.session_token.clone(), None, "settings");
            config_loader = config_loader.credentials_provider(creds);
        }

        let sdk_config = config_loader.load().await;

        //let config = aws_config::load_from_env().await;
        let s3_client = aws_sdk_s3::Client::new(&sdk_config);

        MediaStorageClient {
            s3_client,
            media_bucket
        }
    }
    pub async fn get_media(&self, id: &str) -> anyhow::Result<StoredData> {
        let key = key_for_size(id);
        self.get(&key).await
    }

    async fn get(&self, key: &str) -> anyhow::Result<StoredData> {
        let resp = self.s3_client.get_object()
            .bucket(&self.media_bucket)
            .key(key)
            .send().await?;

        let mime = resp.content_type.unwrap_or("application/octet-stream".to_string());
        let bytes = resp.body;

        Ok(StoredData { bytes, mime })
    }

    pub async fn save_media(&self, id: &str, byte_stream: ByteStream, content_type: &str) -> anyhow::Result<()> {
        let key = key_for_size(id);
        self.save(&key, byte_stream, content_type).await?;
        Ok(())
    }

    async fn save(&self, key: &str, byte_stream: ByteStream, content_type: &str) -> anyhow::Result<()> {
        let _resp = self.s3_client.put_object()
            .bucket(&self.media_bucket)
            .key(key)
            .content_type(content_type)
            .body(byte_stream)
            .send()
            .await?;

        Ok(())
    }

}

fn key_for_size(asset_id: &str) -> String {
    format!("media/{}", asset_id)
}

#[tokio::test]
async fn test_minio() {
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    let resp = s3_client.create_bucket()
        .bucket("myfiles")
        .send()
        .await;

    println!("{resp:?}");
}
