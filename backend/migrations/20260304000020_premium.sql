-- Premium subscription tracking
ALTER TABLE users ADD COLUMN IF NOT EXISTS premium_since TIMESTAMPTZ;
