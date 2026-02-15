use crate::models::{CreateSurveyRequest, SurveyCategory, SurveyDetails, SurveyRecord};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, types::Json, Pool, Postgres, QueryBuilder};

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

#[derive(Debug, sqlx::FromRow)]
struct SurveyRecordRow {
    pub id: String,
    pub start_point: String,
    pub end_point: String,
    pub orientation: String,
    pub distance: f64,
    pub top_distance: String,
    pub category: String,
    pub details: Json<SurveyDetails>,
    pub photo_urls: Vec<String>,
    pub awaiting_photo_count: i32,
    pub remarks: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

fn parse_category(value: &str) -> SurveyCategory {
    serde_json::from_str::<SurveyCategory>(&format!("\"{}\"", value))
        .unwrap_or(SurveyCategory::Unknown)
}

impl SurveyRecordRow {
    fn into_record(self) -> SurveyRecord {
        SurveyRecord {
            id: self.id,
            start_point: self.start_point,
            end_point: self.end_point,
            orientation: self.orientation,
            distance: self.distance,
            top_distance: self.top_distance,
            category: parse_category(&self.category),
            details: self.details,
            photo_urls: self.photo_urls,
            awaiting_photo_count: self.awaiting_photo_count,
            remarks: self.remarks,
            created_at: self.created_at,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct SurveyQueryFilters {
    pub category: Option<String>,
    pub start_point: Option<String>,
    pub end_point: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_surveys(
    pool: &Pool<Postgres>,
    filters: SurveyQueryFilters,
) -> Result<Vec<SurveyRecord>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM survey_records");
    let mut has_where = false;

    if let Some(category) = filters.category {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push("category = ").push_bind(category);
    }

    if let Some(start_point) = filters.start_point {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push("start_point = ").push_bind(start_point);
    }

    if let Some(end_point) = filters.end_point {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push("end_point = ").push_bind(end_point);
    }

    if let Some(created_from) = filters.created_from {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push("created_at >= ").push_bind(created_from);
    }

    if let Some(created_to) = filters.created_to {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push("created_at <= ").push_bind(created_to);
    }

    qb.push(" ORDER BY created_at DESC");

    let mut limit = filters.limit.unwrap_or(50);
    if limit <= 0 {
        limit = 50;
    }
    if limit > 200 {
        limit = 200;
    }
    qb.push(" LIMIT ").push_bind(limit);

    if let Some(offset) = filters.offset {
        if offset > 0 {
            qb.push(" OFFSET ").push_bind(offset);
        }
    }

    let rows = qb.build_query_as::<SurveyRecordRow>().fetch_all(pool).await?;
    Ok(rows.into_iter().map(SurveyRecordRow::into_record).collect())
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

    let result =
        sqlx::query_as::<_, SurveyRecordRow>("SELECT * FROM survey_records WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(result.map(SurveyRecordRow::into_record))
}
