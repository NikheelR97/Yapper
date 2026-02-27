-- Parental control system (metadata-only — E2EE is inviolable)

CREATE TABLE parent_child_relationships (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_user_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    child_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (parent_user_id, child_user_id)
);

CREATE INDEX idx_parent_child ON parent_child_relationships (parent_user_id);

-- Pending friend requests that require parental approval
CREATE TABLE pending_friend_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requester_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requester_name  TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'approved', 'declined')),
    reviewed_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pending_friends_child ON pending_friend_requests (child_user_id, status);

-- Pending server joins that require parental approval
CREATE TABLE pending_server_joins (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    server_name     TEXT NOT NULL,
    invite_code     TEXT,
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'approved', 'declined')),
    reviewed_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pending_joins_child ON pending_server_joins (child_user_id, status);

-- Parent notifications (real-time + persistent)
CREATE TABLE parent_notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_user_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    child_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type            TEXT NOT NULL,  -- 'friend_request', 'server_join', 'milestone'
    reference_id    UUID,           -- pending_friend_requests.id or pending_server_joins.id
    read            BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Screen time data from iOS DeviceActivity + Android UsageStatsManager
CREATE TABLE screen_time_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    app_name        TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    platform        TEXT NOT NULL CHECK (platform IN ('ios', 'android', 'web', 'desktop')),
    recorded_date   DATE NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, app_name, recorded_date, platform)
);

-- Audit log for parental actions (COPPA compliance)
CREATE TABLE parental_action_audit (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_user_id  UUID NOT NULL REFERENCES users(id),
    child_user_id   UUID NOT NULL REFERENCES users(id),
    action          TEXT NOT NULL,  -- 'approve_friend', 'decline_friend', 'approve_join', etc.
    reference_id    UUID,
    ip_address      INET,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
