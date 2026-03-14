use crate::auth;
use crate::database::{self, AppState, SurveyQueryFilters};
use crate::models::{ApiResponse, CreateSurveyRequest};
use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
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

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn create_survey_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateSurveyRequest>,
) -> Response {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    match database::create_survey_record(&state.db, payload).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(ApiResponse {
                success: true,
                message: "Survey record created".to_string(),
                internal_id: Some(id),
            }),
        )
            .into_response(),
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
                .into_response()
        }
    }
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

// ── Photo blob handlers ───────────────────────────────────────────────────────

/// POST /api/surveys/:id/photos
/// Upload one or more photos for a survey via multipart/form-data.
/// Each part should have content-disposition with a `name` and optional `filename`.
pub async fn upload_photo_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(survey_id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    match database::get_survey(&state.db, &survey_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "Survey not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to check survey: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    let mut uploaded_ids: Vec<String> = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                tracing::error!("Multipart error: {:?}", e);
                return (StatusCode::BAD_REQUEST, "Failed to read multipart data").into_response();
            }
        };

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "photo".to_string());
        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(e) => {
                tracing::error!("Failed to read field bytes: {:?}", e);
                return (StatusCode::BAD_REQUEST, "Failed to read upload data").into_response();
            }
        };

        match database::create_photo(&state.db, &survey_id, &filename, &content_type, data).await {
            Ok(id) => uploaded_ids.push(id),
            Err(e) => {
                tracing::error!("Failed to save photo: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to save photo",
                )
                    .into_response();
            }
        }
    }

    if uploaded_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                message: "No files were uploaded".to_string(),
                internal_id: None,
            }),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "photo_ids": uploaded_ids,
        })),
    )
        .into_response()
}

/// GET /api/surveys/:id/photos
/// Return metadata list for all photos belonging to a survey.
pub async fn list_photos_handler(
    State(state): State<AppState>,
    Path(survey_id): Path<String>,
) -> Response {
    match database::list_photos(&state.db, &survey_id).await {
        Ok(photos) => (StatusCode::OK, Json(photos)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list photos: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch photos").into_response()
        }
    }
}

/// GET /api/photos/:photo_id
/// Stream the binary data for a single photo.
pub async fn get_photo_handler(
    State(state): State<AppState>,
    Path(photo_id): Path<String>,
) -> Response {
    match database::get_photo_data(&state.db, &photo_id).await {
        Ok(Some((record, data))) => {
            let disposition = format!("inline; filename=\"{}\"", record.filename);
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, record.content_type),
                    (header::CONTENT_DISPOSITION, disposition),
                ],
                Body::from(data),
            )
                .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Photo not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get photo: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch photo").into_response()
        }
    }
}

/// DELETE /api/photos/:photo_id
/// Remove a photo (requires auth).
pub async fn delete_photo_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(photo_id): Path<String>,
) -> Response {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    match database::delete_photo(&state.db, &photo_id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                message: "Photo deleted".to_string(),
                internal_id: Some(photo_id),
            }),
        )
            .into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Photo not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete photo: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete photo",
            )
                .into_response()
        }
    }
}
