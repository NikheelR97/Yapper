-- ═══════════════════════════════════════════════════════════════════════════════
-- Canvas Expansion — music queue, enhanced polls, clip interactions, live events
-- ═══════════════════════════════════════════════════════════════════════════════

-- ─── Music: extend canvas_music_state ────────────────────────────────────────

ALTER TABLE canvas_music_state
    ADD COLUMN duration_secs    INTEGER CHECK (duration_secs IS NULL
                                            OR (duration_secs > 0
                                                AND duration_secs <= 86400)),
    ADD COLUMN started_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN current_track_id UUID;
-- FK added after music_queue table exists (below).

-- ─── Music: queue ────────────────────────────────────────────────────────────

CREATE TABLE music_queue (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    added_by        UUID NOT NULL REFERENCES users(id),
    artist          TEXT NOT NULL
                        CHECK (char_length(artist) BETWEEN 1 AND 200),
    title           TEXT NOT NULL
                        CHECK (char_length(title) BETWEEN 1 AND 200),
    duration_secs   INTEGER NOT NULL
                        CHECK (duration_secs > 0 AND duration_secs <= 86400),
    source_url      TEXT CHECK (source_url IS NULL
                                OR char_length(source_url) <= 2048),
    album_art_url   TEXT CHECK (album_art_url IS NULL
                                OR char_length(album_art_url) <= 2048),
    position        INTEGER NOT NULL,
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Active queue lookup (upcoming tracks, ordered).
CREATE INDEX idx_music_queue_active
    ON music_queue (server_id, position)
    WHERE started_at IS NULL AND finished_at IS NULL;

-- Now-playing lookup (at most one per server).
CREATE INDEX idx_music_queue_now_playing
    ON music_queue (server_id)
    WHERE started_at IS NOT NULL AND finished_at IS NULL;

-- History lookup (most recent finished tracks).
CREATE INDEX idx_music_queue_history
    ON music_queue (server_id, finished_at DESC)
    WHERE finished_at IS NOT NULL;

-- Now add the FK from canvas_music_state.
ALTER TABLE canvas_music_state
    ADD CONSTRAINT fk_current_track
    FOREIGN KEY (current_track_id) REFERENCES music_queue(id)
    ON DELETE SET NULL;

-- ─── Music: per-server settings ──────────────────────────────────────────────

CREATE TABLE canvas_music_settings (
    server_id           UUID PRIMARY KEY
                            REFERENCES servers(id) ON DELETE CASCADE,
    skip_threshold_pct  SMALLINT NOT NULL DEFAULT 50
                            CHECK (skip_threshold_pct >= 10
                                   AND skip_threshold_pct <= 100),
    queue_enabled       BOOLEAN NOT NULL DEFAULT TRUE
);

-- ─── Music: DJ permission ────────────────────────────────────────────────────

CREATE TABLE canvas_dj_users (
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    granted_by  UUID NOT NULL REFERENCES users(id),
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (server_id, user_id)
);

-- ─── Music: skip votes ──────────────────────────────────────────────────────

CREATE TABLE music_skip_votes (
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (server_id, user_id)
);

-- ─── Polls: extend existing table ───────────────────────────────────────────

ALTER TABLE polls
    ADD COLUMN poll_type  TEXT NOT NULL DEFAULT 'multiple_choice'
                              CHECK (poll_type IN ('binary',
                                                   'multiple_choice',
                                                   'emoji_reaction')),
    ADD COLUMN status     TEXT NOT NULL DEFAULT 'active'
                              CHECK (status IN ('active', 'closed')),
    ADD COLUMN anonymous  BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN closed_at  TIMESTAMPTZ,
    ADD COLUMN closed_by  UUID REFERENCES users(id);

CREATE INDEX idx_polls_active
    ON polls (channel_id, created_at DESC)
    WHERE status = 'active';

-- ─── Clips: reactions ────────────────────────────────────────────────────────
-- E2EE BOUNDARY NOTE: Reactions are plaintext metadata. The emoji and user_id
-- are stored unencrypted. The Clip *content* (ciphertext in messages table)
-- remains E2EE. Reactions are observable by the server and all members.

CREATE TABLE clip_reactions (
    clip_id     UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    emoji       TEXT NOT NULL
                    CHECK (char_length(emoji) BETWEEN 1 AND 8),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (clip_id, user_id, emoji)
);

CREATE INDEX idx_clip_reactions_clip ON clip_reactions (clip_id);

-- ─── Clips: pinning ─────────────────────────────────────────────────────────

CREATE TABLE pinned_clips (
    server_id   UUID NOT NULL REFERENCES servers(id)  ON DELETE CASCADE,
    clip_id     UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    pinned_by   UUID NOT NULL REFERENCES users(id),
    pinned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (server_id, clip_id)
);

-- ─── Canvas events (countdown / live) ────────────────────────────────────────

CREATE TABLE canvas_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    title       TEXT NOT NULL
                    CHECK (char_length(title) BETWEEN 1 AND 200),
    description TEXT
                    CHECK (description IS NULL
                           OR char_length(description) <= 1000),
    event_at    TIMESTAMPTZ NOT NULL,
    created_by  UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_canvas_events_server
    ON canvas_events (server_id, event_at DESC);
