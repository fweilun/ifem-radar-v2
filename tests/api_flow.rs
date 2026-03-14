//! End-to-end API flow test (replaces the old MinIO-based flow).
//!
//! Run with: `cargo test --features integration --test api_flow`
//!
//! Requires a running PostgreSQL instance reachable via DATABASE_URL.
// Imports are conditionally used depending on the `integration` feature.
#![cfg_attr(not(feature = "integration"), allow(unused))]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use anyhow::Context;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use http_body_util::BodyExt;
use ifem_radar_v2::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails};
use ifem_radar_v2::{create_router, database};
use sqlx::postgres::PgPoolOptions;
use std::env;
use tower::ServiceExt;

fn set_default_env(key: &str, value: &str) {
    if env::var_os(key).is_none() {
        env::set_var(key, value);
    }
}

async fn setup() -> database::AppState {
    set_default_env(
        "DATABASE_URL",
        "postgres://ifemfweilun:P@ssw0rdIfem@localhost:5432/ifem_radar",
    );

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    database::AppState { db: pool }
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
        // ── Login ─────────────────────────────────────────────────────────────
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

        // ── Create survey ─────────────────────────────────────────────────────
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
            "create survey failed: {}",
            create_response.status()
        );
        created_survey_id = Some(survey_id.clone());

        // ── Upload photo as blob ───────────────────────────────────────────────
        let boundary = "----AFlowBoundary42";
        let photo_bytes = b"fake_photo_content";
        let mut body_bytes: Vec<u8> = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"photo.jpg\"\r\n\
             Content-Type: image/jpeg\r\n\
             \r\n",
            boundary = boundary,
        )
        .into_bytes();
        body_bytes.extend_from_slice(photo_bytes);
        body_bytes.extend_from_slice(
            format!("\r\n--{boundary}--\r\n", boundary = boundary).as_bytes(),
        );

        let upload_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/surveys/{}/photos", survey_id))
                    .header("authorization", format!("Bearer {}", token))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body_bytes))
                    .unwrap(),
            )
            .await
            .context("photo upload request")?;

        anyhow::ensure!(
            upload_response.status() == StatusCode::CREATED,
            "photo upload failed: {}",
            upload_response.status()
        );

        let upload_body = upload_response.into_body().collect().await?.to_bytes();
        let upload_json: serde_json::Value = serde_json::from_slice(&upload_body)?;
        let photo_id = upload_json["photo_ids"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing photo_id in upload response"))?
            .to_string();

        // ── List photos ───────────────────────────────────────────────────────
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/surveys/{}/photos", survey_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(
            list_response.status() == StatusCode::OK,
            "list photos failed"
        );

        let list_body = list_response.into_body().collect().await?.to_bytes();
        let list_json: serde_json::Value = serde_json::from_slice(&list_body)?;
        let photos = list_json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected array from list photos"))?;
        anyhow::ensure!(photos.len() == 1, "expected 1 photo in list");

        // ── Download photo ────────────────────────────────────────────────────
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/photos/{}", photo_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(
            get_response.status() == StatusCode::OK,
            "get photo failed"
        );

        let returned = get_response.into_body().collect().await?.to_bytes();
        anyhow::ensure!(
            returned.as_ref() == photo_bytes,
            "returned photo bytes mismatch"
        );

        // ── Delete photo ──────────────────────────────────────────────────────
        let delete_response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/photos/{}", photo_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        anyhow::ensure!(
            delete_response.status() == StatusCode::OK,
            "delete photo failed"
        );

        Ok(())
    }
    .await;

    // ── Cleanup ───────────────────────────────────────────────────────────────
    if let Some(survey_id) = &created_survey_id {
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
