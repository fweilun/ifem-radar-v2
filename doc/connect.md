# API Connection Guide

This document explains how to connect to the current endpoints, the login + upload workflow, and how to initialize user accounts.

**Base URL**
- Local: `http://localhost:8080`
- Health check: `GET /health`

**Auth Model**
- Login returns a JWT.
- Protected endpoints require `Authorization: Bearer <token>`.
- Protected endpoints: `POST /api/surveys`, `POST /api/surveys/:id/photos`.

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

### 3) Upload Photo(s)
- Endpoint: `POST /api/surveys/:id/photos`
- Accepts multipart form-data with field name `file`.
- Multiple files can be included in one request (handler iterates all fields).

Request:
```bash
curl -X POST http://localhost:8080/api/surveys/<SURVEY_ID>/photos \
  -H "Authorization: Bearer <JWT>" \
  -F "file=@./path/to/photo.jpg;type=image/jpeg"
```
Response:
```json
{
  "success": true,
  "message": "Uploaded 1 photos",
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
- `details`:
  - `diameter`, `length`, `width`, `protrusion`, `siltation_depth`, `crossing_pipe_count`
  - `change_of_area` (object `{ width, height, change_width, change_height }`)
  - `issues` (array of strings)
- `remarks` (optional)
- `awaiting_photo_count` (int)
