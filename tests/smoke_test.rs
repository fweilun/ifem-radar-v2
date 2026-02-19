use anyhow::Context;
use reqwest::{header::CONTENT_TYPE, multipart, StatusCode};
use serde_json::json;
use std::env;

fn smoke_base_url() -> String {
    if let Ok(v) = env::var("SMOKE_BASE_URL") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }

    if let Ok(v) = env::var("DEPLOY_BASE_URL") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }

    "http://localhost:8080".to_string()
}

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_smoke_flow() {
    // Step 1: Initialize HTTP client and base URL.
    let base_url = smoke_base_url();
    let client = reqwest::Client::new();

    // Step 2: Use fixed smoke-test credentials (no direct Postgres operations).
    let account = "alice";
    let password = "P@ssw0rd";

    let test_result: Result<(), anyhow::Error> = async {
        // Step 3: Verify health endpoint.
        let health_response = client.get(format!("{}/health", base_url)).send().await?;
        anyhow::ensure!(
            health_response.status() == StatusCode::OK,
            "health check failed with status {}",
            health_response.status()
        );

        let health_body = health_response.text().await?;
        anyhow::ensure!(health_body == "OK", "health response is not OK");

        // Step 4: Login and extract JWT token.
        let login_response = client
            .post(format!("{}/api/login", base_url))
            .json(&json!({
                "account": account,
                "password": password
            }))
            .send()
            .await?;

        let login_status = login_response.status();
        if login_status != StatusCode::OK {
            let body = login_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "login failed: status={} body={}",
                login_status,
                body
            ));
        }

        let login_json: serde_json::Value = login_response.json().await?;
        let token = login_json["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing access_token"))?
            .to_string();

        // Step 5: Create a survey with one pending photo.
        let survey_id = uuid::Uuid::new_v4().to_string();
        let create_response = client
            .post(format!("{}/api/surveys", base_url))
            .bearer_auth(&token)
            .json(&json!({
                "id": survey_id,
                "start_point": "SMOKE-A",
                "end_point": "SMOKE-B",
                "orientation": "Left",
                "distance": 1.0,
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
                    "issues": null
                },
                "remarks": "smoke-test",
                "awaiting_photo_count": 1
            }))
            .send()
            .await?;

        let create_status = create_response.status();
        if create_status != StatusCode::CREATED {
            let body = create_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "create survey failed: status={} body={}",
                create_status,
                body
            ));
        }

        // Step 6: Upload one photo to the survey.
        let file_content = "smoke photo content";
        let file_part = multipart::Part::text(file_content.to_string())
            .file_name("smoke.txt")
            .mime_str("text/plain")?;
        let form = multipart::Form::new().part("file", file_part);

        let upload_response = client
            .post(format!("{}/api/surveys/{}/photos", base_url, survey_id))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await?;

        let upload_status = upload_response.status();
        if upload_status != StatusCode::OK {
            let body = upload_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "upload failed: status={} body={}",
                upload_status,
                body
            ));
        }

        // Step 7: Fetch survey and verify stored photo id and awaiting_photo_count == 0.
        let get_survey_response = client
            .get(format!("{}/api/surveys/{}", base_url, survey_id))
            .send()
            .await?;

        let get_survey_status = get_survey_response.status();
        if get_survey_status != StatusCode::OK {
            let body = get_survey_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "get survey failed: status={} body={}",
                get_survey_status,
                body
            ));
        }

        let survey_json: serde_json::Value = get_survey_response.json().await?;
        let awaiting_photo_count = survey_json["awaiting_photo_count"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("missing awaiting_photo_count"))?;
        anyhow::ensure!(
            awaiting_photo_count == 0,
            "expected awaiting_photo_count to be 0 after upload, got {}",
            awaiting_photo_count
        );

        let photo_urls = survey_json["photo_urls"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing photo_urls"))?;
        anyhow::ensure!(
            photo_urls.len() == 1,
            "expected one stored photo id, got {}",
            photo_urls.len()
        );

        let photo_id = photo_urls[0]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("photo id is not string"))?
            .to_string();
        anyhow::ensure!(
            uuid::Uuid::parse_str(&photo_id).is_ok(),
            "stored value is not photo id: {}",
            photo_id
        );

        // Step 8: Fetch photo by id and verify content and content-type.
        let get_photo_response = client
            .get(format!("{}/api/photos/{}", base_url, photo_id))
            .bearer_auth(&token)
            .send()
            .await?;

        let get_photo_status = get_photo_response.status();
        anyhow::ensure!(
            get_photo_status == StatusCode::OK,
            "get photo failed with status {}",
            get_photo_status
        );

        let content_type = get_photo_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        anyhow::ensure!(
            content_type.starts_with("text/plain"),
            "unexpected content-type: {}",
            content_type
        );

        let get_photo_body = get_photo_response.bytes().await?;
        anyhow::ensure!(
            get_photo_body.as_ref() == file_content.as_bytes(),
            "photo content mismatch"
        );

        // Step 9: End smoke flow (no direct DB cleanup in remote mode).
        Ok(())
    }
    .await
    .context("smoke flow failed");

    if let Err(err) = test_result {
        panic!("smoke test failed: {:#}", err);
    }
}
