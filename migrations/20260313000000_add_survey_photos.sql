-- Replace MinIO-based photo storage with PostgreSQL blob storage.

-- Create survey_photos table to store photos as BYTEA blobs.
CREATE TABLE IF NOT EXISTS survey_photos (
    id TEXT PRIMARY KEY,
    survey_id TEXT NOT NULL REFERENCES survey_records(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_survey_photos_survey_id ON survey_photos(survey_id);

-- Remove MinIO-related columns from survey_records.
ALTER TABLE survey_records DROP COLUMN IF EXISTS photo_urls;
ALTER TABLE survey_records DROP COLUMN IF EXISTS awaiting_photo_count;
