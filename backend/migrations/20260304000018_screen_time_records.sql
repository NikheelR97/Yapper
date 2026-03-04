-- Screen time per-app records (S8)
-- Table may already exist if created in a prior session; use IF NOT EXISTS throughout.

CREATE TABLE IF NOT EXISTS screen_time_records (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    app_name          TEXT NOT NULL,
    duration_seconds  INTEGER NOT NULL CHECK (duration_seconds >= 0),
    platform          TEXT NOT NULL CHECK (platform IN ('ios', 'android', 'web', 'desktop')),
    recorded_date     DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, app_name, recorded_date, platform)
);

CREATE INDEX IF NOT EXISTS idx_screen_time_records_user_date
    ON screen_time_records (user_id, recorded_date DESC);
