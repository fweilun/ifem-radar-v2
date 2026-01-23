use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod database;
mod handlers;
mod models;
mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connections
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = database::connect_db(&database_url).await?;

    let s3_client = storage::init_s3_client().await;
    let bucket_name = env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "ifem-radar".to_string());

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app_state = database::AppState {
        db: pool,
        s3_client,
        bucket_name,
    };

    // Router
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/surveys", post(handlers::create_survey_handler))
        .route(
            "/api/surveys/:id/photos",
            post(handlers::upload_photo_handler),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Run
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
