pub mod database;
pub mod handlers;
pub mod models;
pub mod storage;
pub mod auth;

use axum::{
    routing::{get, post},
    Router,
};
use database::AppState;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/login", post(auth::login))
        .route(
            "/api/surveys",
            get(handlers::list_surveys_handler).post(handlers::create_survey_handler),
        )
        .route("/api/surveys/:id", get(handlers::get_survey_handler))
        .route(
            "/api/surveys/:id/photos",
            post(handlers::upload_photo_handler),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
