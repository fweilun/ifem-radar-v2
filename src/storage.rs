use anyhow::{Context, Result};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn public_endpoint_url() -> Option<String> {
    env_non_empty("AWS_PUBLIC_ENDPOINT_URL")
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
    Ok(build_object_url(bucket, key))
}

pub fn build_object_url(bucket: &str, key: &str) -> String {
    let endpoint = public_endpoint_url()
        .or_else(|| env_non_empty("AWS_ENDPOINT_URL"))
        .unwrap_or_default();
    if !endpoint.is_empty() {
        format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, key)
    } else {
        format!("s3://{}/{}", bucket, key)
    }
}

pub fn rewrite_presigned_url(url: &str) -> Result<String> {
    let public_endpoint = public_endpoint_url()
        .ok_or_else(|| anyhow::anyhow!("AWS_PUBLIC_ENDPOINT_URL must be set"))?;

    if !public_endpoint.contains("://") {
        return Err(anyhow::anyhow!(
            "AWS_PUBLIC_ENDPOINT_URL must include scheme (e.g. http:// or https://)"
        ));
    }

    let scheme_end = url
        .find("://")
        .map(|idx| idx + 3)
        .ok_or_else(|| anyhow::anyhow!("presigned url missing scheme"))?;
    let after_scheme = &url[scheme_end..];
    let path_start = after_scheme
        .find('/')
        .ok_or_else(|| anyhow::anyhow!("presigned url missing path"))?;
    let path_and_query = &after_scheme[path_start..];
    Ok(format!(
        "{}{}",
        public_endpoint.trim_end_matches('/'),
        path_and_query
    ))
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
