-- Screen Time settings + daily summaries (S8)

CREATE TABLE screen_time_settings (
    child_user_id        UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    daily_limit_minutes  INTEGER NOT NULL DEFAULT 180 CHECK (daily_limit_minutes >= 30 AND daily_limit_minutes <= 480),
    bedtime_start        TIME,
    bedtime_end          TIME,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE screen_time_daily_summaries (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    summary_date    DATE NOT NULL,
    total_seconds   INTEGER NOT NULL DEFAULT 0 CHECK (total_seconds >= 0),
    yapper_seconds  INTEGER NOT NULL DEFAULT 0 CHECK (yapper_seconds >= 0),
    other_seconds   INTEGER NOT NULL DEFAULT 0 CHECK (other_seconds >= 0),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, summary_date)
);

CREATE INDEX idx_screen_time_records_user_date
    ON screen_time_records (user_id, recorded_date DESC);

CREATE INDEX idx_screen_time_daily_summaries_user_date
    ON screen_time_daily_summaries (user_id, summary_date DESC);
