use crate::database::{self, AppState, SurveyQueryFilters};
use crate::models::{ApiResponse, CreateSurveyRequest};
use crate::storage;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SurveyQueryParams {
    pub category: Option<String>,
    pub start_point: Option<String>,
    pub end_point: Option<String>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn parse_rfc3339(opt: Option<String>) -> Result<Option<DateTime<Utc>>, String> {
    match opt {
        Some(value) => DateTime::parse_from_rfc3339(&value)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(|_| format!("Invalid datetime format: {}", value)),
        None => Ok(None),
    }
}

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

pub async fn get_survey_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match database::get_survey(&state.db, &id).await {
        Ok(Some(record)) => (StatusCode::OK, Json(record)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get survey: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch record",
            )
                .into_response()
        }
    }
}

pub async fn list_surveys_handler(
    State(state): State<AppState>,
    Query(params): Query<SurveyQueryParams>,
) -> impl IntoResponse {
    let created_from = match parse_rfc3339(params.created_from) {
        Ok(value) => value,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };
    let created_to = match parse_rfc3339(params.created_to) {
        Ok(value) => value,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };

    let filters = SurveyQueryFilters {
        category: params.category,
        start_point: params.start_point,
        end_point: params.end_point,
        created_from,
        created_to,
        limit: params.limit,
        offset: params.offset,
    };

    match database::list_surveys(&state.db, filters).await {
        Ok(records) => (StatusCode::OK, Json(records)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list surveys: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch records",
            )
                .into_response()
        }
    }
}
