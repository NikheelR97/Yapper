use sqlx::Row;
use uuid::Uuid;

use crate::AppState;

const SESSION_RETENTION_DAYS: i32 = 7;
const OAUTH_CODE_RETENTION_DAYS: i32 = 7;
const PASSWORD_TOKEN_RETENTION_DAYS: i32 = 1;
const EMAIL_TOKEN_RETENTION_DAYS: i32 = 7;
const DELIVERED_SYNC_RETENTION_DAYS: i32 = 7;
const STALE_SYNC_RETENTION_DAYS: i32 = 30;
const TRUST_REQUEST_RETENTION_DAYS: i32 = 30;
const CONSUMED_OPK_RETENTION_DAYS: i32 = 30;
const REVOKED_DEVICE_BACKUP_RETENTION_DAYS: i32 = 30;
pub const DELETED_ACCOUNT_HOLD_DAYS: i32 = 30;
pub const DELETED_ACCOUNT_BASIS_PENDING: &str = "pending_purge_review";
pub const DELETED_ACCOUNT_BASIS_REFERENTIAL_INTEGRITY: &str = "retained_referential_integrity";

async fn deleted_account_has_blocking_refs(
    user_id: Uuid,
    state: &AppState,
) -> anyhow::Result<bool> {
    let row = sqlx::query(
        "SELECT (
            EXISTS (SELECT 1 FROM messages WHERE sender_id = $1)
            OR EXISTS (SELECT 1 FROM servers WHERE owner_id = $1)
            OR EXISTS (SELECT 1 FROM server_invite_links WHERE creator_id = $1)
            OR EXISTS (SELECT 1 FROM canvas_music_state WHERE set_by_user_id = $1)
            OR EXISTS (SELECT 1 FROM music_queue WHERE added_by = $1)
            OR EXISTS (SELECT 1 FROM canvas_dj_users WHERE granted_by = $1)
            OR EXISTS (SELECT 1 FROM polls WHERE creator_id = $1 OR closed_by = $1)
            OR EXISTS (SELECT 1 FROM server_emojis WHERE created_by = $1)
            OR EXISTS (SELECT 1 FROM pinned_clips WHERE pinned_by = $1)
            OR EXISTS (SELECT 1 FROM canvas_events WHERE created_by = $1)
        ) AS has_blockers",
    )
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await?;

    Ok(row.try_get::<bool, _>("has_blockers")?)
}

async fn finalize_deleted_accounts(state: &AppState) -> anyhow::Result<(u64, u64)> {
    let candidate_rows = sqlx::query(
        "SELECT id
         FROM users
         WHERE deleted_at IS NOT NULL
           AND COALESCE(
               deletion_hold_until,
               deleted_at + ($1::int * INTERVAL '1 day')
           ) <= NOW()",
    )
    .bind(DELETED_ACCOUNT_HOLD_DAYS)
    .fetch_all(state.db.pool())
    .await?;

    let mut hard_deleted_users = 0;
    let mut retained_deleted_shells = 0;

    for row in candidate_rows {
        let user_id: Uuid = row.try_get("id")?;

        if deleted_account_has_blocking_refs(user_id, state).await? {
            // Mark as retained and push the hold window out so this shell
            // is not re-evaluated every cleanup cycle while refs persist.
            retained_deleted_shells += sqlx::query(
                "UPDATE users
                 SET deletion_retention_basis = $2,
                     deletion_hold_until = GREATEST(
                         deletion_hold_until,
                         NOW() + ($3::int * INTERVAL '1 day')
                     )
                 WHERE id = $1
                   AND deleted_at IS NOT NULL
                   AND deletion_retention_basis IS DISTINCT FROM $2",
            )
            .bind(user_id)
            .bind(DELETED_ACCOUNT_BASIS_REFERENTIAL_INTEGRITY)
            .bind(DELETED_ACCOUNT_HOLD_DAYS)
            .execute(state.db.pool())
            .await?
            .rows_affected();
            continue;
        }

        hard_deleted_users +=
            sqlx::query("DELETE FROM users WHERE id = $1 AND deleted_at IS NOT NULL")
                .bind(user_id)
                .execute(state.db.pool())
                .await?
                .rows_affected();
    }

    Ok((hard_deleted_users, retained_deleted_shells))
}

pub async fn run_cleanup(state: &AppState) -> anyhow::Result<()> {
    let pool = state.db.pool();

    let expired_media_rows = sqlx::query(
        "DELETE FROM media_uploads
         WHERE expires_at IS NOT NULL AND expires_at < NOW()
         RETURNING object_key",
    )
    .fetch_all(pool)
    .await?;
    let expired_media_keys: Vec<String> = expired_media_rows
        .iter()
        .filter_map(|row| row.try_get::<String, _>("object_key").ok())
        .collect();

    let deleted_sessions = sqlx::query(
        "DELETE FROM sessions
         WHERE expires_at < NOW() - ($1::int * INTERVAL '1 day')
            OR (revoked_at IS NOT NULL AND revoked_at < NOW() - ($1::int * INTERVAL '1 day'))",
    )
    .bind(SESSION_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let cleared_password_tokens = sqlx::query(
        "UPDATE users
         SET password_reset_token = NULL,
             password_reset_expires_at = NULL
         WHERE password_reset_expires_at IS NOT NULL
           AND password_reset_expires_at < NOW() - ($1::int * INTERVAL '1 day')",
    )
    .bind(PASSWORD_TOKEN_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let cleared_email_tokens = sqlx::query(
        "UPDATE users
         SET email_verify_token = NULL,
             email_verify_expires_at = NULL
         WHERE email_verify_expires_at IS NOT NULL
           AND email_verify_expires_at < NOW() - ($1::int * INTERVAL '1 day')",
    )
    .bind(EMAIL_TOKEN_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted_oauth_codes = sqlx::query(
        "DELETE FROM oauth_login_codes
         WHERE expires_at < NOW()
            OR (consumed_at IS NOT NULL AND consumed_at < NOW() - ($1::int * INTERVAL '1 day'))",
    )
    .bind(OAUTH_CODE_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted_sync_events = sqlx::query(
        "DELETE FROM device_sync_events
         WHERE (delivered_at IS NOT NULL AND delivered_at < NOW() - ($1::int * INTERVAL '1 day'))
            OR (created_at < NOW() - ($2::int * INTERVAL '1 day'))",
    )
    .bind(DELIVERED_SYNC_RETENTION_DAYS)
    .bind(STALE_SYNC_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted_trust_requests = sqlx::query(
        "DELETE FROM device_trust_requests
         WHERE (approved_at IS NOT NULL AND approved_at < NOW() - ($1::int * INTERVAL '1 day'))
            OR (created_at < NOW() - ($1::int * INTERVAL '1 day'))",
    )
    .bind(TRUST_REQUEST_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted_consumed_opks = sqlx::query(
        "DELETE FROM one_time_prekeys
         WHERE consumed = TRUE
           AND created_at < NOW() - ($1::int * INTERVAL '1 day')",
    )
    .bind(CONSUMED_OPK_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted_revoked_backups = sqlx::query(
        "DELETE FROM device_key_backups
         WHERE revoked_at IS NOT NULL
           AND revoked_at < NOW() - ($1::int * INTERVAL '1 day')",
    )
    .bind(REVOKED_DEVICE_BACKUP_RETENTION_DAYS)
    .execute(pool)
    .await?
    .rows_affected();

    let (hard_deleted_users, retained_deleted_shells) = finalize_deleted_accounts(state).await?;

    for r2_key in &expired_media_keys {
        if let Err(error) = crate::media::r2::delete_object(r2_key).await {
            tracing::warn!(r2_key = %r2_key, "Failed to delete expired media object from R2: {error}");
        }
    }

    let deleted_expired_media = expired_media_keys.len() as u64;
    if deleted_sessions > 0
        || cleared_password_tokens > 0
        || cleared_email_tokens > 0
        || deleted_oauth_codes > 0
        || deleted_sync_events > 0
        || deleted_trust_requests > 0
        || deleted_consumed_opks > 0
        || deleted_revoked_backups > 0
        || hard_deleted_users > 0
        || retained_deleted_shells > 0
        || deleted_expired_media > 0
    {
        tracing::info!(
            deleted_sessions,
            cleared_password_tokens,
            cleared_email_tokens,
            deleted_oauth_codes,
            deleted_sync_events,
            deleted_trust_requests,
            deleted_consumed_opks,
            deleted_revoked_backups,
            hard_deleted_users,
            retained_deleted_shells,
            deleted_expired_media,
            "Retention cleanup removed stale operational records"
        );
    }

    Ok(())
}
