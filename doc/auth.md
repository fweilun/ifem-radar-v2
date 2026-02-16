# Auth Design

This document describes the current authentication design and the relevant parts of the codebase, including the actual code paths for JWT creation and password verification.

## Overview
- Auth method: JWT (Bearer token in `Authorization` header).
- Login: `POST /api/login` with `account` + `password`.
- Passwords: stored as Argon2 hashes in `account_info.password_hash`.
- Protected routes: currently `POST /api/surveys` and `POST /api/surveys/:id/photos`.
- JWT claims: `account`, `exp`.

## Data Model
**Table**: `account_info` (see `survey.sql`, `migrations/20260216000000_create_account_info.sql`)

```sql
CREATE TABLE IF NOT EXISTS account_info (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    role TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
```

## JWT Design (Actual Code)
**File**: `src/auth.rs`

### Key setup
JWT keys are built from `JWT_SECRET` (fallback to `dev-secret-change-me`).

```rust
pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set; using insecure default");
        "dev-secret-change-me".to_string()
    });
    Keys::new(secret.as_bytes())
});
```

### Claims
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub account: String,
    pub exp: usize,
}
```

### Creating JWT in login
```rust
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    if payload.account.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    let ok = database::check_account(&state.db, &payload.account, &payload.password)
        .await
        .map_err(|err| {
            tracing::error!("Failed to check account: {:?}", err);
            AuthError::TokenCreation
        })?;

    if !ok {
        return Err(AuthError::WrongCredentials);
    }

    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        account: payload.account,
        exp,
    };

    let token =
        encode(&Header::default(), &claims, &KEYS.encoding).map_err(|_| AuthError::TokenCreation)?;

    Ok(Json(AuthBody::new(token)))
}
```

### Parsing JWT from Header
```rust
pub fn claims_from_headers(headers: &HeaderMap) -> Result<Claims, AuthError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AuthError::InvalidToken)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    let token_data = decode::<Claims>(token, &KEYS.decoding, &Validation::default())
        .map_err(|_| AuthError::InvalidToken)?;

    Ok(token_data.claims)
}
```

## Password Verification (Argon2)
**File**: `src/database.rs`

### `check_account`
```rust
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
```

## Protected Routes
**File**: `src/handlers.rs`

```rust
pub async fn create_survey_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateSurveyRequest>,
) -> impl IntoResponse {
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
        ),
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
        }
    }
}

pub async fn upload_photo_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Err(err) = auth::claims_from_headers(&headers) {
        return err.into_response();
    }

    // ... existing upload flow ...
}
```

## Creating Accounts (Argon2 hash)
**File**: `scripts/create_account.rs`

```rust
let salt = SaltString::generate(&mut OsRng);
let password_hash = Argon2::default()
    .hash_password(password.as_bytes(), &salt)
    .map_err(|e| anyhow::anyhow!("hash password failed: {e}"))?
    .to_string();

sqlx::query(
    r#"
    INSERT INTO account_info (id, account, password_hash, full_name, role, is_active)
    VALUES ($1, $2, $3, $4, $5, TRUE)
    ON CONFLICT (account) DO UPDATE
    SET password_hash = EXCLUDED.password_hash,
        full_name = COALESCE(EXCLUDED.full_name, account_info.full_name),
        role = COALESCE(EXCLUDED.role, account_info.role),
        is_active = TRUE,
        updated_at = CURRENT_TIMESTAMP
    "#,
)
.bind(uuid::Uuid::new_v4().to_string())
.bind(&account)
.bind(password_hash)
.bind(full_name)
.bind(role)
.execute(&pool)
.await?;
```

## Environment Variables
- `JWT_SECRET` (required for secure JWT signing; default is insecure)
- `DATABASE_URL`

## Files Summary
- `src/auth.rs`: JWT handling, login handler, header-based auth parsing
- `src/database.rs`: `check_account` with Argon2 verification
- `src/handlers.rs`: protected routes call `claims_from_headers`
- `survey.sql`, `migrations/20260216000000_create_account_info.sql`: `account_info` schema
- `scripts/create_account.rs`: CLI to create/update accounts
