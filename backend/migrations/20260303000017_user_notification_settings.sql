-- User notification preferences and DND schedule

CREATE TABLE user_notification_settings (
    user_id                UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    push_enabled           BOOLEAN NOT NULL DEFAULT TRUE,
    notify_dms             BOOLEAN NOT NULL DEFAULT TRUE,
    notify_mentions        BOOLEAN NOT NULL DEFAULT TRUE,
    notify_friend_requests BOOLEAN NOT NULL DEFAULT TRUE,
    notify_server_activity BOOLEAN NOT NULL DEFAULT FALSE,
    notify_yap_recordings  BOOLEAN NOT NULL DEFAULT TRUE,
    dnd_enabled            BOOLEAN NOT NULL DEFAULT FALSE,
    dnd_start              TIME,  -- nullable HH:MM:SS, only used when dnd_enabled = TRUE
    dnd_end                TIME,  -- nullable HH:MM:SS, only used when dnd_enabled = TRUE
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
