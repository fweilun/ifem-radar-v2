use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dotenvy::dotenv;
use http_body_util::BodyExt; // for collect
use ifem_radar_v2::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails};
use ifem_radar_v2::{create_router, database, storage};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tower::ServiceExt; // for oneshot

// Helper to setup state for tests
async fn setup() -> database::AppState {
    dotenv().ok();

    // In a real test, you might want to use a separate test DB or transaction.
    // For now we use the local DB but verify connection.
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    let s3_client = storage::init_s3_client().await;
    let bucket_name = env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "ifem-radar".to_string());

    database::AppState {
        db: pool,
        s3_client,
        bucket_name,
    }
}

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

#[tokio::test]
async fn test_create_survey() {
    let state = setup().await;
    let app = create_router(state);

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
        awaiting_photo_count: 2,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/surveys")
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
