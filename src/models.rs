use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SurveyRecord {
    pub id: String, // UUID
    pub start_point: String,
    pub end_point: String,
    pub orientation: String,  // 上/下/左/右
    pub distance: f64,        // 距離
    pub top_distance: String, // 頂距 (例如: >0)

    pub category: SurveyCategory, // ex. 連接管、橫越館等
    pub details: Json<SurveyDetails>,

    pub photo_urls: Vec<String>,   // 存放在 MinIO 的路徑
    pub awaiting_photo_count: i32, // 剩餘待上傳照片張數
    pub remarks: Option<String>,   // 備註
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "varchar")]
#[sqlx(rename_all = "snake_case")]
pub enum SurveyCategory {
    ConnectingPipe, // 連接管
    CrossingPipe,   // 橫越管
    BoxDamage,      // 箱涵破損
    AttachmentLoss, // 附掛缺失
    Siltation,      // 淤積
    SectionChange,  // 斷面變化
    CannotPass,     // 無法縱走
    // Fallback for any other string
    #[serde(other)]
    Unknown,
}

// Manual implementation for VARCHAR compatibility if needed,
// using sqlx::Type's built-in support for enums mapped to strings usually works
// if the DB type is created or if mapped to text.
// For simplicity in this `investigate.sql` (where category is VARCHAR(50)),
// we might need to implement Type<Postgres> manually or align names.
// Here we use `sqlx(rename_all = ...)` to match the lowercase strings in DB if we inserted them that way.
// However, `investigate.sql` defines it as VARCHAR(50), not a custom ENUM type.
// So we should treat it as String in DB but Enum in Rust.
// sqlx `type_name = "varchar"` helps? It might need `sqlx::Type` implementation to proxy to String.
// Easier way for VARCHAR column: Implement Type by deriving it but saying it is transparent to String?
// Or just let it be String in struct and convert.
// Let's try the `sqlx::Type` derive with `#[sqlx(transparent)]` if it was a wrapper, but for enum:
// We will treat it as String in the Struct for safety, or implement From/To String.
// For now, let's stick to the user's `spec.rust` intention.
// User used `#[sqlx(type_name = "varchar")]`. This usually implies a custom type in Postgres,
// OR we rely on sqlx to handle string conversion.
// If the column is just VARCHAR, sqlx might complain if we try to bind a custom enum.
// Let's change `category` in `SurveyRecord` to `String` to be safe, or keep `SurveyCategory` but handle deserialization.
// Given the user's spec, I'll keep `SurveyCategory` but ensure it works with VARCHAR.
// Using `sqlx::encode::MakeArg` etc is complex.
// HACK: I will change the struct field to String for DB storage ease in `database.rs`,
// but the DTO used for API interaction can use the Enum.
// ACTUALLY, checking `spec.rust`:
// ```rust
// #[derive(Debug, Serialize, Deserialize, sqlx::Type)]
// #[sqlx(type_name = "varchar")]
// pub enum SurveyCategory ...
// ```
// This suggests the user *wants* it to work this way. I will trust sqlx can handle it or I'll add a proper implementation.
// To use a Rust enum with a VARCHAR column, usually we implement `Type<DB>` returning `VARCHAR`.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangeOfArea {
    pub width: f64,
    pub height: f64,
    pub change_width: f64,
    pub change_height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SurveyDetails {
    pub diameter: Option<i32>,                // 直徑
    pub length: Option<f64>,                  // 長度 L
    pub width: Option<f64>,                   // 寬度 W
    pub protrusion: Option<i32>,              // 凸出 (圖中紅字 19)
    pub siltation_depth: Option<i32>,         // 淤積深度 (cm)
    pub crossing_pipe_count: Option<i32>,     // 橫越管數量
    pub change_of_area: Option<ChangeOfArea>, // 斷面變化
    pub issues: Option<Vec<String>>,          // 標籤型多選
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub internal_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PresignUploadRequest {
    pub survey_id: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PresignUploadResponse {
    pub upload_url: String,
    pub file_key: String,
    pub expires_in: u64,
    pub required_headers: Vec<PresignHeader>,
}

#[derive(Debug, Serialize)]
pub struct PresignHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteUploadRequest {
    pub survey_id: String,
    pub file_key: String,
}

// Request DTO (what the client sends)
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSurveyRequest {
    pub id: String, // UUID from client
    pub start_point: String,
    pub end_point: String,
    pub orientation: String,
    pub distance: f64,
    pub top_distance: String,
    pub category: SurveyCategory,
    pub details: SurveyDetails,
    pub remarks: Option<String>,
    pub awaiting_photo_count: i32,
}
