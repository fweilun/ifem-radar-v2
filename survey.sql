-- Survey records table
CREATE TABLE IF NOT EXISTS survey_records (
    id TEXT PRIMARY KEY,
    start_point TEXT NOT NULL,
    end_point TEXT NOT NULL,
    orientation VARCHAR(20) NOT NULL,
    distance FLOAT8 NOT NULL,
    top_distance VARCHAR(50) NOT NULL,
    category VARCHAR(50) NOT NULL,
    details JSONB NOT NULL,
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_survey_category ON survey_records(category);
CREATE INDEX IF NOT EXISTS idx_survey_created_at ON survey_records(created_at);

-- Account info table
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

CREATE INDEX IF NOT EXISTS idx_account_info_account ON account_info(account);

-- Photo blobs table (replaces MinIO storage)
CREATE TABLE IF NOT EXISTS survey_photos (
    id TEXT PRIMARY KEY,
    survey_id TEXT NOT NULL REFERENCES survey_records(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_survey_photos_survey_id ON survey_photos(survey_id);
