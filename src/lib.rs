pub mod database;
pub mod handlers;
pub mod models;
pub mod storage;

use axum::{
    routing::{get, post},
    Router,
};
use database::AppState;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/surveys", post(handlers::create_survey_handler))
        .route(
            "/api/surveys/:id/photos",
            post(handlers::upload_photo_handler),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
