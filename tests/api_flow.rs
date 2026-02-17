use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use anyhow::Context;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use http_body_util::BodyExt;
use ifem_radar_v2::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails};
use ifem_radar_v2::{create_router, database, storage};
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::env;
use tower::ServiceExt;

async fn ensure_bucket(
    client: &aws_sdk_s3::Client,
    bucket: &str,
) -> Result<(), anyhow::Error> {
    if client.head_bucket().bucket(bucket).send().await.is_ok() {
        return Ok(());
    }

    match client.create_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(()),
        Err(err) => {
            let err_str = format!("{err:?}");
            if err_str.contains("BucketAlreadyOwnedByYou")
                || err_str.contains("BucketAlreadyExists")
            {
                return Ok(());
            }
            Err(anyhow::anyhow!("create bucket failed: {}", err_str))
        }
    }
}

fn set_default_env(key: &str, value: &str) {
    if env::var_os(key).is_none() {
        env::set_var(key, value);
    }
}

async fn setup() -> database::AppState {
    // Defaults to work with `docker compose up -d` on localhost.
    set_default_env(
        "DATABASE_URL",
        "postgres://ifemfweilun:P@ssw0rdIfem@localhost:5432/ifem_radar",
    );
    set_default_env("AWS_ENDPOINT_URL", "http://localhost:9000");
    set_default_env("AWS_ACCESS_KEY_ID", "minioadmin");
    set_default_env("AWS_SECRET_ACCESS_KEY", "minioadmin");
    set_default_env("AWS_REGION", "us-east-1");
    set_default_env("AWS_BUCKET_NAME", "ifem-radar");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    let s3_client = storage::init_s3_client().await;
    let bucket_name = env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "ifem-radar-test".to_string());

    database::AppState {
        db: pool,
        s3_client,
        bucket_name,
    }
}

fn extract_key_from_url(url: &str, bucket: &str) -> Option<String> {
    if let Some(stripped) = url.strip_prefix("s3://") {
        let mut parts = stripped.splitn(2, '/');
        let bucket_part = parts.next()?;
        let key_part = parts.next()?;
        if bucket_part == bucket {
            return Some(key_part.to_string());
        }
        return None;
    }

    let needle = format!("/{}/", bucket);
    let idx = url.find(&needle)?;
    Some(url[(idx + needle.len())..].to_string())
}

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_api_flow_happy_path() {
    let state = setup().await;
    let app = create_router(state.clone());

    let account = format!("test_{}", uuid::Uuid::new_v4());
    let password = "P@ssw0rd!";

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO account_info (id, account, password_hash, is_active)
        VALUES ($1, $2, $3, TRUE)
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&account)
    .bind(&password_hash)
    .execute(&state.db)
    .await
    .expect("Failed to insert test account");

    let mut created_survey_id: Option<String> = None;

    let test_result: Result<(), anyhow::Error> = async {
        ensure_bucket(&state.s3_client, &state.bucket_name)
            .await
            .context("ensure bucket")?;

        let login_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "account": account,
                            "password": password
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(login_response.status() == StatusCode::OK, "login failed");

        let body = login_response.into_body().collect().await?.to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body)?;
        let token = body_json["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing access_token"))?
            .to_string();

        let survey_id = uuid::Uuid::new_v4().to_string();
        let payload = CreateSurveyRequest {
            id: survey_id.clone(),
            start_point: "A".to_string(),
            end_point: "B".to_string(),
            orientation: "Left".to_string(),
            distance: 10.5,
            top_distance: ">0".to_string(),
            category: SurveyCategory::ConnectingPipe,
            details: SurveyDetails {
                diameter: Some(100),
                length: None,
                width: None,
                protrusion: None,
                siltation_depth: None,
                crossing_pipe_count: None,
                change_of_area: None,
                issues: Some(vec!["Crack".to_string()]),
            },
            remarks: Some("Test remark".to_string()),
            awaiting_photo_count: 1,
        };

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/surveys")
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload)?))
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(
            create_response.status() == StatusCode::CREATED,
            "create survey failed"
        );
        created_survey_id = Some(survey_id.clone());

        let upload_url_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/surveys/upload-url")
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "survey_id": survey_id,
                            "filename": "test_photo.txt",
                            "content_type": "text/plain"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(
            upload_url_response.status() == StatusCode::OK,
            "create upload url failed"
        );
        let body = upload_url_response
            .into_body()
            .collect()
            .await?
            .to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body)?;
        let upload_url = body_json["upload_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing upload_url"))?
            .to_string();
        let file_key = body_json["file_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing file_key"))?
            .to_string();
        let content_type = body_json["required_headers"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["name"] == "Content-Type"))
            .and_then(|item| item["value"].as_str())
            .unwrap_or("application/octet-stream")
            .to_string();

        let http_client = Client::new();
        let put_response = http_client
            .put(&upload_url)
            .header("Content-Type", content_type)
            .body("fake image content")
            .send()
            .await
            .context("presigned upload request failed")?;

        anyhow::ensure!(
            put_response.status().is_success(),
            "presigned upload failed"
        );

        let complete_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/surveys/complete")
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "survey_id": survey_id,
                            "file_key": file_key
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await?;

        let complete_status = complete_response.status();
        if complete_status != StatusCode::OK {
            let body = complete_response.into_body().collect().await?.to_bytes();
            let body_str = String::from_utf8_lossy(&body);
            return Err(anyhow::anyhow!(
                "complete upload failed: status={} body={}",
                complete_status,
                body_str
            ));
        }

        Ok(())
    }
    .await;

    if let Some(survey_id) = &created_survey_id {
        if let Ok(Some(record)) = database::get_survey(&state.db, survey_id).await {
            let mut keys = HashSet::new();
            for url in record.photo_urls {
                if let Some(key) = extract_key_from_url(&url, &state.bucket_name) {
                    keys.insert(key);
                }
            }

            for key in keys {
                let _ = state
                    .s3_client
                    .delete_object()
                    .bucket(&state.bucket_name)
                    .key(key)
                    .send()
                    .await;
            }
        }

        let _ = sqlx::query("DELETE FROM survey_records WHERE id = $1")
            .bind(survey_id)
            .execute(&state.db)
            .await;
    }

    let _ = sqlx::query("DELETE FROM account_info WHERE account = $1")
        .bind(&account)
        .execute(&state.db)
        .await;

    if let Err(err) = test_result {
        panic!("api flow test failed: {:#}", err);
    }
}
