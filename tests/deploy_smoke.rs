//! Deployment smoke test against a running server.
//!
//! Run with:
//!   DEPLOY_BASE_URL=http://localhost:8080 \
//!   DEPLOY_ACCOUNT=admin DEPLOY_PASSWORD=admin \
//!   cargo test --test deploy_smoke -- --ignored
//!
//! Set `DEPLOY_RUN_FULL=1` to exercise the full photo upload/download/delete flow.

use dotenvy::dotenv;
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
                "issues": ["Crack"]
            },
            "remarks": "Deploy smoke test"
        }))
        .send()
        .await
        .expect("create survey request failed");

    assert!(create_resp.status().is_success());

    // 4) Upload photo as blob via multipart POST
    let photo_bytes = b"smoke test photo content";
    let photo_part = reqwest::multipart::Part::bytes(photo_bytes.to_vec())
        .file_name("smoke.jpg")
        .mime_str("image/jpeg")
        .expect("invalid mime");
    let form = reqwest::multipart::Form::new().part("file", photo_part);

    let upload_resp = client
        .post(format!("{}/api/surveys/{}/photos", base_url, survey_id))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("photo upload request failed");

    let upload_status = upload_resp.status();
    if !upload_status.is_success() {
        let body = upload_resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        panic!(
            "photo upload failed: status={} body={}",
            upload_status, body
        );
    }

    let upload_body: serde_json::Value = upload_resp
        .json()
        .await
        .expect("upload response json parse failed");
    let photo_id = upload_body["photo_ids"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .expect("photo_ids[0] missing")
        .to_string();
    println!("deploy_smoke: photo_id={}", photo_id);

    // 5) List photos for survey
    let list_resp = client
        .get(format!("{}/api/surveys/{}/photos", base_url, survey_id))
        .send()
        .await
        .expect("list photos request failed");
    assert!(list_resp.status().is_success());
    let list_body: serde_json::Value = list_resp
        .json()
        .await
        .expect("list photos json parse failed");
    let photos = list_body.as_array().expect("expected array");
    assert!(!photos.is_empty(), "expected at least one photo");

    // 6) Download photo and verify content
    let get_resp = client
        .get(format!("{}/api/photos/{}", base_url, photo_id))
        .send()
        .await
        .expect("get photo request failed");
    assert!(get_resp.status().is_success());
    let returned_bytes = get_resp
        .bytes()
        .await
        .expect("get photo body failed");
    assert_eq!(
        returned_bytes.as_ref(),
        photo_bytes,
        "downloaded photo bytes mismatch"
    );

    // 7) Delete photo
    let del_resp = client
        .delete(format!("{}/api/photos/{}", base_url, photo_id))
        .bearer_auth(&token)
        .send()
        .await
        .expect("delete photo request failed");
    assert!(del_resp.status().is_success());

    // 8) Verify survey record is still accessible
    let survey_resp = client
        .get(format!("{}/api/surveys/{}", base_url, survey_id))
        .send()
        .await
        .expect("get survey request failed");
    assert!(
        survey_resp.status().is_success(),
        "get survey failed after photo deletion"
    );
}
