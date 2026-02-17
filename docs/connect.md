# API Connection Guide

This document explains the current endpoints, the login + upload workflow, and how to initialize user accounts.

**Base URL**
- Local: `http://localhost:8080`
- Health check: `GET /health` (returns `OK`)

**Auth Model**
- Login returns a JWT (expires in 24 hours).
- Protected endpoints require `Authorization: Bearer <token>`.
- Protected endpoints: `POST /api/surveys`, `POST /api/surveys/upload-url`, `POST /api/surveys/complete`.
- Public endpoints: `GET /api/surveys`, `GET /api/surveys/<SURVEY_ID>`, `GET /health`.

## Workflow (Happy Path)
### 1) Login
Request:
```bash
curl -X POST http://localhost:8080/api/login \
  -H "Content-Type: application/json" \
  -d '{"account":"YOUR_ACCOUNT","password":"YOUR_PASSWORD"}'
```
Response:
```json
{
  "access_token": "<JWT>",
  "token_type": "Bearer"
}
```

### 2) Create Survey
Returns `201 Created` on success.

Request:
```bash
curl -X POST http://localhost:8080/api/surveys \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{
    "id":"<UUID>",
    "start_point":"A",
    "end_point":"B",
    "orientation":"Left",
    "distance":10.5,
    "top_distance":">0",
    "category":"ConnectingPipe",
    "details":{
      "diameter":100,
      "length":null,
      "width":null,
      "protrusion":null,
      "siltation_depth":null,
      "crossing_pipe_count":null,
      "change_of_area":null,
      "issues":["Crack"]
    },
    "remarks":"Test remark",
    "awaiting_photo_count":2
  }'
```
Response:
```json
{
  "success": true,
  "message": "Survey record created",
  "internal_id": "<UUID>"
}
```

### 3) Upload Photo(s) via Presigned PUT (Recommended)
This flow avoids pushing large files through the backend.

#### 3.1 Request presigned upload URL
Endpoint: `POST /api/surveys/upload-url`

Required fields: `survey_id`, `filename`.
Optional fields: `content_type` (default `application/octet-stream`), `expires_in` (seconds, clamped to 60..=3600, default 900).

Request:
```bash
curl -X POST http://localhost:8080/api/surveys/upload-url \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{
    "survey_id":"<SURVEY_ID>",
    "filename":"photo.jpg",
    "content_type":"image/jpeg",
    "expires_in":900
  }'
```

Response:
```json
{
  "upload_url": "<PRESIGNED_PUT_URL>",
  "file_key": "surveys/<SURVEY_ID>/<UUID>.jpg",
  "expires_in": 900,
  "required_headers": [
    { "name": "Content-Type", "value": "image/jpeg" }
  ]
}
```

#### 3.2 Upload directly to MinIO/S3
Use the `required_headers` from the response.

```bash
curl -X PUT "<PRESIGNED_PUT_URL>" \
  -H "Content-Type: image/jpeg" \
  --data-binary @./path/to/photo.jpg
```

#### 3.3 Notify backend upload completed
Endpoint: `POST /api/surveys/complete`

The `file_key` must start with `surveys/<SURVEY_ID>/` and should be the value returned from step 3.1.

```bash
curl -X POST http://localhost:8080/api/surveys/complete \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{
    "survey_id":"<SURVEY_ID>",
    "file_key":"surveys/<SURVEY_ID>/<UUID>.jpg"
  }'
```

Response:
```json
{
  "success": true,
  "message": "Photo upload completed",
  "internal_id": "<SURVEY_ID>"
}
```

## Query Endpoints
### Get Survey by ID
```bash
curl http://localhost:8080/api/surveys/<SURVEY_ID>
```

### List Surveys
Supported query params: `category`, `start_point`, `end_point`, `created_from`, `created_to`, `limit`, `offset`.
- `created_from` / `created_to` must be RFC3339 (e.g., `2024-01-01T00:00:00Z`).
- `limit` defaults to 50 and is capped at 200.
- `offset` is applied only when greater than 0.

```bash
curl "http://localhost:8080/api/surveys?limit=20&offset=0"
curl "http://localhost:8080/api/surveys?category=ConnectingPipe&limit=20"
curl "http://localhost:8080/api/surveys?created_from=2024-01-01T00:00:00Z&created_to=2024-12-31T23:59:59Z&limit=50"
```

## Create/Initialize Accounts
### CLI (Recommended)
Use the built-in script to create or update an account with Argon2 hashing:

```bash
cargo run --bin create_account -- <account> <password> [full_name] [role]
```
Example:
```bash
cargo run --bin create_account -- alice P@ssw0rd "Alice Chen" admin
```

### Notes
- Passwords are stored as Argon2 hashes in `account_info.password_hash`.
- Login validates the password against the stored Argon2 hash.

## Reference: Request Body Fields
**Create survey body** (`CreateSurveyRequest`):
- `id` (UUID string)
- `start_point`, `end_point`, `orientation`, `distance`, `top_distance`
- `category`: `ConnectingPipe`, `CrossingPipe`, `BoxDamage`, `AttachmentLoss`, `Siltation`, `SectionChange`, `CannotPass`, `Unknown`
- `details`: object with `diameter`, `length`, `width`, `protrusion`, `siltation_depth`, `crossing_pipe_count`, `change_of_area` (object `{ width, height, change_width, change_height }`), `issues` (array of strings, optional)
- `remarks` (optional)
- `awaiting_photo_count` (int)
