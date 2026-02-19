use anyhow::{Context, Result};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client};

pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub async fn init_s3_client() -> Client {
    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("us-east-1"));
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // If using MinIO, we need to adjust endpoint_url and force_path_style
    // This usually comes from ENV variables AWS_ENDPOINT_URL.
    // aws_config automatically picks up standard AWS env vars.
    // For MinIO specifically, we often need:
    // AWS_ENDPOINT_URL=http://localhost:9000
    // AWS_ACCESS_KEY_ID=minioadmin
    // AWS_SECRET_ACCESS_KEY=minioadmin
    // AWS_REGION=us-east-1

    // We'll check if we need to enforce path style (common for MinIO).
    let endpoint = std::env::var("AWS_ENDPOINT_URL").unwrap_or_default();

    let builder = aws_sdk_s3::config::Builder::from(&config);
    let builder = if !endpoint.is_empty() {
        builder.force_path_style(true)
    } else {
        builder
    };

    let s3_config = builder.build();
    Client::from_conf(s3_config)
}

pub async fn upload_file(
    client: &Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<String> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(data))
        .content_type(content_type)
        .send()
        .await
        .context("Failed to upload to S3")?;

    // Return the URL or Key.
    // Constructing URL depends on setup (public URL vs internal).
    // For now, return the key or a constructed path.
    // If endpoint is set, we might prepend it.
    let endpoint = std::env::var("AWS_ENDPOINT_URL").unwrap_or_default();
    if !endpoint.is_empty() {
        Ok(format!("{}/{}/{}", endpoint, bucket, key))
    } else {
        Ok(format!("s3://{}/{}", bucket, key))
    }
}

pub async fn get_file(client: &Client, bucket: &str, key: &str) -> Result<Option<StoredObject>> {
    match client.get_object().bucket(bucket).key(key).send().await {
        Ok(output) => {
            let content_type = output.content_type().map(ToString::to_string);
            let bytes = output
                .body
                .collect()
                .await
                .context("Failed to read object body")?
                .into_bytes()
                .to_vec();

            Ok(Some(StoredObject {
                bytes,
                content_type,
            }))
        }
        Err(err) => {
            let err_str = err.to_string();
            if err_str.contains("NoSuchKey") || err_str.contains("NotFound") {
                return Ok(None);
            }
            Err(anyhow::anyhow!("Failed to get object: {}", err_str))
        }
    }
}
