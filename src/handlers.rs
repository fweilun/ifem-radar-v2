use crate::auth;
use crate::database::{self, AppState, SurveyQueryFilters};
use crate::models::{
    ApiResponse, CompleteUploadRequest, CreateSurveyRequest, PresignHeader, PresignUploadRequest,
    PresignUploadResponse,
};
use crate::storage;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::path::Path as FsPath;
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

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn create_upload_url_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<PresignUploadRequest>,
) -> Response {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    if payload.survey_id.trim().is_empty() || payload.filename.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                message: "survey_id and filename are required".to_string(),
                internal_id: None,
            }),
        )
            .into_response();
    }

    match database::get_survey(&state.db, &payload.survey_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Survey not found").into_response();
        }
        Err(e) => {
            tracing::error!("Failed to check survey: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check survey",
            )
                .into_response();
        }
    }

    let ext = FsPath::new(&payload.filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();

    let file_key = format!(
        "surveys/{}/{}{}",
        payload.survey_id,
        uuid::Uuid::new_v4(),
        ext
    );

    let expires_in = payload.expires_in.unwrap_or(900).clamp(60, 3600);
    let content_type = payload
        .content_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let upload_url = match storage::presign_put_url(
        &state.s3_client,
        &state.bucket_name,
        &file_key,
        Some(&content_type),
        expires_in,
    )
    .await
    {
        Ok(url) => url,
        Err(e) => {
            tracing::error!("Failed to presign upload url: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create upload url",
            )
                .into_response();
        }
    };

    let response = PresignUploadResponse {
        upload_url,
        file_key,
        expires_in,
        required_headers: vec![PresignHeader {
            name: "Content-Type".to_string(),
            value: content_type.to_string(),
        }],
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn complete_upload_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CompleteUploadRequest>,
) -> Response {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    if payload.survey_id.trim().is_empty() || payload.file_key.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                message: "survey_id and file_key are required".to_string(),
                internal_id: None,
            }),
        )
            .into_response();
    }

    let expected_prefix = format!("surveys/{}/", payload.survey_id);
    if !payload.file_key.starts_with(&expected_prefix) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                message: "file_key does not match survey_id".to_string(),
                internal_id: None,
            }),
        )
            .into_response();
    }

    match database::get_survey(&state.db, &payload.survey_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Survey not found").into_response();
        }
        Err(e) => {
            tracing::error!("Failed to check survey: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check survey",
            )
                .into_response();
        }
    }

    let url = storage::build_object_url(&state.bucket_name, &payload.file_key);
    if let Err(e) = database::add_photo_url(&state.db, &payload.survey_id, &url).await {
        tracing::error!("Failed to update DB for photo: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                success: false,
                message: "Failed to update database".to_string(),
                internal_id: None,
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Photo upload completed".to_string(),
            internal_id: Some(payload.survey_id),
        }),
    )
        .into_response()
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
