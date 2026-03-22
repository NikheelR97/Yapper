-- Push notification tokens: store FCM/APNS tokens per device.
-- A device can have at most one active push token.

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS push_token TEXT,
    ADD COLUMN IF NOT EXISTS push_platform TEXT CHECK (push_platform IN ('fcm', 'apns', 'web'));

CREATE INDEX IF NOT EXISTS idx_devices_push_token
    ON devices (push_token) WHERE push_token IS NOT NULL;
