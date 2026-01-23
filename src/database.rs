use crate::models::{CreateSurveyRequest, SurveyRecord};
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, types::Json, Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub s3_client: aws_sdk_s3::Client,
    pub bucket_name: String,
}

pub async fn connect_db(database_url: &str) -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn create_survey_record(
    pool: &Pool<Postgres>,
    req: CreateSurveyRequest,
) -> Result<String> {
    // Serialize category to string to ensure compatibility with VARCHAR(50)
    // or rely on sqlx implementation if configured correctly.
    // Here we act safe and just cast via serde or Debug/Display if available,
    // but assuming CreateSurveyRequest used the Enum.
    // We can use serde_json::to_value to get the string representation if the Enum is unit-only.
    // Or just impl ToString. Let's rely on serde serialization to string.
    let category_str = serde_json::to_string(&req.category)?
        .trim_matches('"')
        .to_string();

    let rec = sqlx::query!(
        r#"
        INSERT INTO survey_records (
            id, start_point, end_point, orientation, distance, top_distance,
            category, details, awaiting_photo_count, remarks
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
        req.id,
        req.start_point,
        req.end_point,
        req.orientation,
        req.distance,
        req.top_distance,
        category_str,
        Json(&req.details) as _, // Force sqlx to treat this as JSONB compatible
        req.awaiting_photo_count,
        req.remarks
    )
    .fetch_one(pool)
    .await?;

    Ok(rec.id)
}

pub async fn add_photo_url(pool: &Pool<Postgres>, id: &str, url: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE survey_records
        SET photo_urls = array_append(photo_urls, $2),
            awaiting_photo_count = GREATEST(awaiting_photo_count - 1, 0)
        WHERE id = $1
        "#,
        id,
        url
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn get_survey(pool: &Pool<Postgres>, id: &str) -> Result<Option<SurveyRecord>> {
    // We need to query. Since SurveyCategory is an enum,
    // we assume we can read it back as string and cast,
    // or we use query_as! if types match.
    // However, sqlx macros check DB types. The DB type is VARCHAR.
    // The struct type is SurveyCategory.
    // Automated mapping might fail if sqlx doesn't know how to go VARCHAR -> Enum.
    // We'll use manual query_as or just query and map.
    // For simplicity, let's use `sqlx::query_as` which is runtime-checked (mostly)
    // or defining a manual row mapper is safer.

    // Actually, let's try `query_as` with the struct details.
    // Note: This requires SurveyCategory to impl sqlx::Type<Postgres> and accept VARCHAR.
    // If not, this might fail at runtime.
    // Given usage of `sqlx::Type` in models.rs, it should be fine IF the type names matched
    // OR if transparent is used.
    // But let's proceed.

    let result = sqlx::query_as::<_, SurveyRecord>("SELECT * FROM survey_records WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(result)
}
