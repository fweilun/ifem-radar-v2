# iFEM Radar V2 Backend

## Overview
This is the backend service for iFEM Radar V2, built with Rust, Axum, SQLx, and AWS SDK for S3. It handles survey record creation and photo uploads via MinIO.

## Prerequisites
- Rust (latest stable)
- PostgreSQL (running on port 5432)
- MinIO (S3 compatible storage)

## Setup

1. **Environment Variables**:
   Check `.env` file. It works with default `minioadmin` credentials and a local Postgres instance.

2. **Database Initialization**:
   This project uses `sqlx` macros which check the database at compile time. You must create the database and schema first.
   Run the helper script:
   ```bash
   cargo run --bin setup_db
   ```
   This will:
   - Create the `ifem_radar` database if it doesn't exist.
   - Apply the necessary SQL schema (migrations).

3. **Running the Server**:
   ```bash
   cargo run --bin ifem-radar-v2
   ```
   The server will listen on port 8080.

## API Endpoints

- **POST /api/surveys**: Create a new survey record.
  - Body: JSON matching `CreateSurveyRequest`.
- **POST /api/surveys/:id/photos**: Upload photos for a survey.
  - Body: Multipart form data with file fields.

## Project Structure
- `src/main.rs`: Entry point and routing.
- `src/handlers.rs`: HTTP request handlers.
- `src/models.rs`: Data structures (DTOs) and DB models.
- `src/database.rs`: Database logic (Postgres/SQLx).
- `src/storage.rs`: Storage logic (MinIO/S3).
- `migrations/`: SQL migration files.
