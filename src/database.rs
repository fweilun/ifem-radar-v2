use crate::models::{CreateSurveyRequest, PhotoRecord, SurveyCategory, SurveyDetails, SurveyRecord};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, types::Json, Pool, Postgres, QueryBuilder};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

pub async fn connect_db(database_url: &str) -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn check_account(
    pool: &Pool<Postgres>,
    username: &str,
    password: &str,
) -> Result<bool> {
    let stored_hash: Option<String> = sqlx::query_scalar(
        r#"
        SELECT password_hash
        FROM account_info
        WHERE account = $1 AND is_active = TRUE
        LIMIT 1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    let Some(stored_hash) = stored_hash else {
        return Ok(false);
    };

    let parsed = match PasswordHash::new(&stored_hash) {
        Ok(parsed) => parsed,
        Err(err) => {
            tracing::error!("Invalid password hash for account {}: {:?}", username, err);
            return Ok(false);
        }
    };

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub async fn create_survey_record(
    pool: &Pool<Postgres>,
    req: CreateSurveyRequest,
) -> Result<String> {
    let category_str = serde_json::to_string(&req.category)?
        .trim_matches('"')
        .to_string();

    let rec: (String,) = sqlx::query_as(
        r#"
        INSERT INTO survey_records (
            id, start_point, end_point, orientation, distance, top_distance,
            category, details, remarks
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(&req.id)
    .bind(&req.start_point)
    .bind(&req.end_point)
    .bind(&req.orientation)
    .bind(req.distance)
    .bind(&req.top_distance)
    .bind(&category_str)
    .bind(Json(&req.details))
    .bind(&req.remarks)
    .fetch_one(pool)
    .await?;

    Ok(rec.0)
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

    let _ = has_where;

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

pub async fn get_survey(pool: &Pool<Postgres>, id: &str) -> Result<Option<SurveyRecord>> {
    let result =
        sqlx::query_as::<_, SurveyRecordRow>("SELECT * FROM survey_records WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(result.map(SurveyRecordRow::into_record))
}

// ── Photo blob CRUD ──────────────────────────────────────────────────────────

/// Persist a photo blob and return its generated ID.
pub async fn create_photo(
    pool: &Pool<Postgres>,
    survey_id: &str,
    filename: &str,
    content_type: &str,
    data: Vec<u8>,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO survey_photos (id, survey_id, filename, content_type, data)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&id)
    .bind(survey_id)
    .bind(filename)
    .bind(content_type)
    .bind(&data)
    .execute(pool)
    .await?;
    Ok(id)
}

/// List photo metadata for a survey (no blob data).
pub async fn list_photos(pool: &Pool<Postgres>, survey_id: &str) -> Result<Vec<PhotoRecord>> {
    let rows = sqlx::query_as::<_, PhotoRecord>(
        r#"
        SELECT id, survey_id, filename, content_type, created_at
        FROM survey_photos
        WHERE survey_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(survey_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch a single photo's metadata and binary data.
pub async fn get_photo_data(
    pool: &Pool<Postgres>,
    photo_id: &str,
) -> Result<Option<(PhotoRecord, Vec<u8>)>> {
    #[derive(sqlx::FromRow)]
    struct PhotoRow {
        id: String,
        survey_id: String,
        filename: String,
        content_type: String,
        data: Vec<u8>,
        created_at: Option<DateTime<Utc>>,
    }

    let row = sqlx::query_as::<_, PhotoRow>(
        r#"
        SELECT id, survey_id, filename, content_type, data, created_at
        FROM survey_photos
        WHERE id = $1
        "#,
    )
    .bind(photo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        (
            PhotoRecord {
                id: r.id,
                survey_id: r.survey_id,
                filename: r.filename,
                content_type: r.content_type,
                created_at: r.created_at,
            },
            r.data,
        )
    }))
}

/// Delete a photo by ID. Returns `true` if a row was deleted.
pub async fn delete_photo(pool: &Pool<Postgres>, photo_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM survey_photos WHERE id = $1")
        .bind(photo_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
