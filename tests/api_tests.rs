use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dotenvy::dotenv;
use http_body_util::BodyExt; // for collect
use ifem_radar_v2::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails};
use ifem_radar_v2::{create_router, database, storage};
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
    let bucket_name = env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "ifem-radar-test".to_string());

    database::AppState {
        db: pool,
        s3_client,
        bucket_name,
    }
}

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

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_upload_photo() {
    let state = setup().await;
    let app = create_router(state.clone());

    // 1. Create a survey record first
    let survey_id = uuid::Uuid::new_v4().to_string();
    let payload = CreateSurveyRequest {
        id: survey_id.clone(),
        start_point: "Start".to_string(),
        end_point: "End".to_string(),
        orientation: "Up".to_string(),
        distance: 5.0,
        top_distance: "0.5".to_string(),
        category: SurveyCategory::Siltation,
        details: SurveyDetails {
            diameter: None,
            length: None,
            width: None,
            protrusion: None,
            siltation_depth: Some(10),
            crossing_pipe_count: None,
            change_of_area: None,
            issues: None,
        },
        remarks: None,
        awaiting_photo_count: 1,
    };

    database::create_survey_record(&state.db, payload)
        .await
        .expect("Failed to create survey record");

    // 2. Construct Multipart Body
    let boundary = "------------------------14737809831466499882746641449";
    let file_content = "fake image content";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test_photo.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {file_content}\r\n\
         --{boundary}--\r\n",
        boundary = boundary,
        file_content = file_content
    );

    // 3. Send Request
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/surveys/{}/photos", survey_id))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // 4. Verify Response
    assert_eq!(response.status(), StatusCode::OK);

    // let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    // let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // assert_eq!(body_json["success"], true);

    // // 5. Verify DB Update
    // let record = database::get_survey(&state.db, &survey_id)
    //     .await
    //     .unwrap()
    //     .unwrap();
    // assert!(!record.photo_urls.is_empty());
    // assert!(record.photo_urls[0].contains("test_photo.txt"));
    // // Awaiting count should prevent dropping below 0, but we started with 1, so it should be 0 now
    // assert_eq!(record.awaiting_photo_count, 0);
}
