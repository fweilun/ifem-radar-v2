use crate::database::{self, AppState};
use crate::models::{ApiResponse, CreateSurveyRequest};
use crate::storage;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

pub async fn create_survey_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateSurveyRequest>,
) -> impl IntoResponse {
    match database::create_survey_record(&state.db, payload).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(ApiResponse {
                success: true,
                message: "Survey record created".to_string(),
                internal_id: Some(id),
            }),
        ),
        Err(e) => {
            tracing::error!("Failed to create survey: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to create record: {}", e),
                    internal_id: None,
                }),
            )
        }
    }
}

pub async fn upload_photo_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Basic verification if record exists could be done here

    let mut uploaded_urls = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = if let Some(name) = field.file_name() {
            name.to_string()
        } else {
            uuid::Uuid::new_v4().to_string() + ".jpg"
        };

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        match field.bytes().await {
            Ok(data) => {
                // Generate a unique key: surveys/{id}/{uuid}-{filename}
                let key = format!("surveys/{}/{}", id, file_name);

                match storage::upload_file(
                    &state.s3_client,
                    &state.bucket_name,
                    &key,
                    data.to_vec(),
                    &content_type,
                )
                .await
                {
                    Ok(url) => {
                        // Update DB
                        if let Err(e) = database::add_photo_url(&state.db, &id, &url).await {
                            tracing::error!("Failed to update DB for photo: {:?}", e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse {
                                    success: false,
                                    message: "Failed to update database".to_string(),
                                    internal_id: None,
                                }),
                            );
                        }
                        uploaded_urls.push(url);
                    }
                    Err(e) => {
                        tracing::error!("Failed to upload photo to S3: {:?}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse {
                                success: false,
                                message: "Failed to upload file".to_string(),
                                internal_id: None,
                            }),
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to read field bytes: {:?}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse {
                        success: false,
                        message: "Failed to read file".to_string(),
                        internal_id: None,
                    }),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: format!("Uploaded {} photos", uploaded_urls.len()),
            internal_id: Some(id),
        }),
    )
}

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
