use anyhow::{Context, Result};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

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
    Ok(build_object_url(bucket, key))
}

pub fn build_object_url(bucket: &str, key: &str) -> String {
    let endpoint = std::env::var("AWS_ENDPOINT_URL").unwrap_or_default();
    if !endpoint.is_empty() {
        format!("{}/{}/{}", endpoint, bucket, key)
    } else {
        format!("s3://{}/{}", bucket, key)
    }
}

pub async fn presign_put_url(
    client: &Client,
    bucket: &str,
    key: &str,
    content_type: Option<&str>,
    expires_in_secs: u64,
) -> Result<String> {
    let mut req = client.put_object().bucket(bucket).key(key);
    if let Some(content_type) = content_type {
        req = req.content_type(content_type);
    }

    let config = PresigningConfig::expires_in(Duration::from_secs(expires_in_secs))?;
    let presigned = req.presigned(config).await?;
    Ok(presigned.uri().to_string())
}
