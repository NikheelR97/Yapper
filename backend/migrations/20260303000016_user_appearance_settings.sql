-- User appearance preferences (theme, font size, density, motion)

CREATE TABLE user_appearance_settings (
    user_id       UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    theme         TEXT    NOT NULL DEFAULT 'dark',        -- 'dark' | 'oled' | 'light'
    font_size     INTEGER NOT NULL DEFAULT 14,            -- 12–18 px
    density       TEXT    NOT NULL DEFAULT 'comfortable', -- 'comfortable' | 'compact'
    reduce_motion BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
