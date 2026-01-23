use dotenvy::dotenv;
use ifem_radar_v2::{create_router, database, storage};
use std::env;
use std::net::SocketAddr;

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

    let app = create_router(app_state);

    // Run
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
