DROP INDEX IF EXISTS idx_devices_user_installation;

CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_user_installation
    ON devices (user_id, installation_id)
    WHERE installation_id IS NOT NULL AND revoked_at IS NULL;
