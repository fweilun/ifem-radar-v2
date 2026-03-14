//! Integration smoke tests.
//!
//! Run with: `cargo test --features integration --test api_tests`
//!
//! Requires a running PostgreSQL instance. Set `DATABASE_URL` (and optionally
//! `AUTH_ACCOUNT` / `AUTH_PASSWORD`) before running.

// Imports are conditionally used depending on the `integration` feature.
#![cfg_attr(not(feature = "integration"), allow(unused))]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use dotenvy::dotenv;
use http_body_util::BodyExt;
use ifem_radar_v2::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails};
use ifem_radar_v2::{create_router, database};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use tower::ServiceExt;

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn setup() -> database::AppState {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");
    database::AppState { db: pool }
}

#[cfg(feature = "integration")]
async fn ensure_test_account(pool: &Pool<Postgres>, account: &str, password: &str) {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO account_info (id, account, password_hash, is_active)
        VALUES ($1, $2, $3, TRUE)
        ON CONFLICT (account) DO UPDATE
        SET password_hash = EXCLUDED.password_hash,
            is_active = TRUE
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(account)
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("Failed to upsert test account");
}

#[cfg(feature = "integration")]
async fn login_token(state: &database::AppState) -> String {
    let app = create_router(state.clone());
    let account = env::var("AUTH_ACCOUNT").unwrap_or_else(|_| "admin".to_string());
    let password = env::var("AUTH_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    ensure_test_account(&state.db, &account, &password).await;

    let response = app
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
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    body_json["access_token"]
        .as_str()
        .expect("access_token must be string")
        .to_string()
}

#[cfg(feature = "integration")]
fn make_survey_request(survey_id: &str) -> CreateSurveyRequest {
    CreateSurveyRequest {
        id: survey_id.to_string(),
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
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_health_check() {
    let state = setup().await;
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_create_survey() {
    let state = setup().await;
    let token = login_token(&state).await;
    let app = create_router(state);

    let survey_id = uuid::Uuid::new_v4().to_string();
    let payload = make_survey_request(&survey_id);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/surveys")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["success"], true);
    assert_eq!(body_json["internal_id"], survey_id);
}

/// Test the full photo blob lifecycle: upload → list → download → delete.
#[cfg(feature = "integration")]
#[tokio::test]
async fn test_photo_blob_lifecycle() {
    let state = setup().await;
    let token = login_token(&state).await;

    // 1. Create a survey to attach photos to.
    let survey_id = uuid::Uuid::new_v4().to_string();
    database::create_survey_record(&state.db, make_survey_request(&survey_id))
        .await
        .expect("Failed to create survey record");

    // 2. Upload a photo via multipart POST.
    let boundary = "----TestBoundary12345";
    let photo_bytes: &[u8] = b"\xFF\xD8\xFF\xE0fake_jpeg_data";
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

    let upload_response = create_router(state.clone())
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
        .unwrap();

    assert_eq!(upload_response.status(), StatusCode::CREATED);
    let upload_body = upload_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();
    let photo_ids = upload_json["photo_ids"].as_array().expect("photo_ids array");
    assert_eq!(photo_ids.len(), 1);
    let photo_id = photo_ids[0].as_str().unwrap().to_string();

    // 3. List photos for the survey.
    let list_response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/surveys/{}/photos", survey_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let list_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    let photos = list_json.as_array().expect("photo array");
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0]["id"], photo_id);
    assert_eq!(photos[0]["filename"], "photo.jpg");

    // 4. Download the photo and verify the binary content.
    let get_response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/{}", photo_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let returned_data = get_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(returned_data.as_ref(), photo_bytes);

    // 5. Delete the photo.
    let delete_response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/photos/{}", photo_id))
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // 6. Confirm the photo is gone.
    let gone_response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/{}", photo_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(gone_response.status(), StatusCode::NOT_FOUND);
}

/// Uploading to a non-existent survey should return 404.
#[cfg(feature = "integration")]
#[tokio::test]
async fn test_upload_photo_survey_not_found() {
    let state = setup().await;
    let token = login_token(&state).await;

    let boundary = "----TestBoundary99";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"x.jpg\"\r\n\
         Content-Type: image/jpeg\r\n\
         \r\nnoop\r\n--{boundary}--\r\n",
        boundary = boundary,
    );

    let response = create_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/surveys/nonexistent-id/photos")
                .header("authorization", format!("Bearer {}", token))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
