use dotenvy::dotenv;
use ifem_radar_v2::storage;
use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::test]
#[ignore]
async fn test_deploy_smoke() {
    dotenv().ok();
    let base_url = env::var("DEPLOY_BASE_URL")
        .expect("DEPLOY_BASE_URL is required, e.g. http://localhost:8080");
    let account = env::var("DEPLOY_ACCOUNT").expect("DEPLOY_ACCOUNT is required");
    let password = env::var("DEPLOY_PASSWORD").expect("DEPLOY_PASSWORD is required");
    let run_full = env::var("DEPLOY_RUN_FULL").unwrap_or_default() == "1";

    let client = Client::new();

    // 1) Health check
    let health_resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("health request failed");
    assert!(health_resp.status().is_success());

    // 2) Login
    let login_resp = client
        .post(format!("{}/api/login", base_url))
        .json(&json!({
            "account": account,
            "password": password,
        }))
        .send()
        .await
        .expect("login request failed");

    assert!(login_resp.status().is_success());
    let login_body: serde_json::Value = login_resp
        .json()
        .await
        .expect("login response json parse failed");
    let token = login_body["access_token"]
        .as_str()
        .expect("access_token missing")
        .to_string();

    if !run_full {
        return;
    }

    // 3) Create survey
    let survey_id = uuid::Uuid::new_v4().to_string();
    println!("deploy_smoke: survey_id={}", survey_id);
    let create_resp = client
        .post(format!("{}/api/surveys", base_url))
        .bearer_auth(&token)
        .json(&json!({
            "id": survey_id,
            "start_point": "A",
            "end_point": "B",
            "orientation": "Left",
            "distance": 10.5,
            "top_distance": ">0",
            "category": "ConnectingPipe",
            "details": {
                "diameter": 100,
                "length": null,
                "width": null,
                "protrusion": null,
                "siltation_depth": null,
                "crossing_pipe_count": null,
                "change_of_area": null,
                "issues": ["Crack"]
            },
            "remarks": "Deploy smoke test",
            "awaiting_photo_count": 1
        }))
        .send()
        .await
        .expect("create survey request failed");

    assert!(create_resp.status().is_success());

    // 4) Get presigned upload URL
    let upload_url_resp = client
        .post(format!("{}/api/surveys/upload-url", base_url))
        .bearer_auth(&token)
        .json(&json!({
            "survey_id": survey_id,
            "filename": "smoke.txt",
            "content_type": "text/plain"
        }))
        .send()
        .await
        .expect("upload url request failed");

    let upload_status = upload_url_resp.status();
    if !upload_status.is_success() {
        let body = upload_url_resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        panic!(
            "upload url request failed: status={} body={}",
            upload_status, body
        );
    }
    let upload_url_body: serde_json::Value = upload_url_resp
        .json()
        .await
        .expect("upload url response json parse failed");
    let upload_url = upload_url_body["upload_url"]
        .as_str()
        .expect("upload_url missing");
    let file_key = upload_url_body["file_key"]
        .as_str()
        .expect("file_key missing")
        .to_string();
    println!("deploy_smoke: file_key={}", file_key);
    let content_type = upload_url_body["required_headers"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == "Content-Type"))
        .and_then(|item| item["value"].as_str())
        .unwrap_or("application/octet-stream");

    // 5) Upload directly to MinIO/S3
    let put_resp = client
        .put(upload_url)
        .header("Content-Type", content_type)
        .body("smoke test")
        .send()
        .await
        .expect("presigned upload request failed");

    assert!(put_resp.status().is_success());

    // 6) Complete upload
    let complete_resp = client
        .post(format!("{}/api/surveys/complete", base_url))
        .bearer_auth(&token)
        .json(&json!({
            "survey_id": survey_id,
            "file_key": file_key
        }))
        .send()
        .await
        .expect("complete upload request failed");

    assert!(complete_resp.status().is_success());

    // 7) Verify upload recorded in survey
    let survey_resp = client
        .get(format!("{}/api/surveys/{}", base_url, survey_id))
        .send()
        .await
        .expect("get survey request failed");

    let survey_status = survey_resp.status();
    if !survey_status.is_success() {
        let body = survey_resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        panic!(
            "get survey failed: status={} body={}",
            survey_status, body
        );
    }

    let survey_body: serde_json::Value = survey_resp
        .json()
        .await
        .expect("survey response json parse failed");
    let photo_urls = survey_body["photo_urls"]
        .as_array()
        .expect("photo_urls missing or not array");
    let found = photo_urls.iter().any(|item| {
        item.as_str()
            .map(|url| url.contains(&file_key))
            .unwrap_or(false)
    });
    assert!(found, "uploaded file_key not found in photo_urls");

    // 8) Verify object exists in MinIO/S3
    let bucket = env::var("AWS_BUCKET_NAME").expect("AWS_BUCKET_NAME is required for minio check");
    println!("deploy_smoke: bucket={}", bucket);
    let s3_client = storage::init_s3_client().await;
    if let Err(err) = s3_client
        .head_object()
        .bucket(&bucket)
        .key(&file_key)
        .send()
        .await
    {
        panic!("minio head_object failed: {:?}", err);
    }
}
