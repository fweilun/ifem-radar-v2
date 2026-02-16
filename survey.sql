-- Create table for survey records
CREATE TABLE IF NOT EXISTS survey_records (
    id TEXT PRIMARY KEY, -- Using TEXT for UUID to match Rust String (or UUID type)
    
    start_point TEXT NOT NULL,
    end_point TEXT NOT NULL,
    orientation VARCHAR(20) NOT NULL, -- 上/下/左/右
    distance FLOAT8 NOT NULL,
    top_distance VARCHAR(50) NOT NULL,
    
    category VARCHAR(50) NOT NULL,
    
    details JSONB NOT NULL,
    
    photo_urls TEXT[] NOT NULL DEFAULT '{}',
    
    awaiting_photo_count INT NOT NULL DEFAULT 0,
    
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_survey_category ON survey_records(category);
CREATE INDEX IF NOT EXISTS idx_survey_created_at ON survey_records(created_at);

CREATE TABLE IF NOT EXISTS account_info (
    id TEXT PRIMARY KEY, -- Using TEXT for UUID to match Rust String (or UUID type)
    account TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    role TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_account_info_account ON account_info(account);
