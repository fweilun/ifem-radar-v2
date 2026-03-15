pub mod auth;
pub mod database;
pub mod handlers;
pub mod models;

use axum::{
    http::{header::{AUTHORIZATION, CONTENT_TYPE}, Method},
    routing::{get, post},
    Router,
};

use database::AppState;
use tower_http::trace::TraceLayer;
use tower_http::cors::{Any, CorsLayer};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
    .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/login", post(auth::login))
        .route(
            "/api/surveys",
            get(handlers::list_surveys_handler).post(handlers::create_survey_handler),
        )
        .route("/api/surveys/:id", get(handlers::get_survey_handler))
        // Photo blob endpoints
        .route(
            "/api/surveys/:id/photos",
            post(handlers::upload_photo_handler).get(handlers::list_photos_handler),
        )
        .route(
            "/api/photos/:photo_id",
            get(handlers::get_photo_handler).delete(handlers::delete_photo_handler),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
