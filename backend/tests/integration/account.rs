//! Integration tests for account deletion and deleted-account retention cleanup.

use super::{
    authorization_header_name, build_test_state, create_test_user, csrf_header_name,
    csrf_header_value,
};
use axum_test::TestServer;
use serial_test::serial;
use sqlx::Row;
use uuid::Uuid;
use yapper_server::{build_router, retention};

async fn build_account_test_server() -> Option<(yapper_server::AppState, TestServer)> {
    let state = build_test_state().await?;
    let server =
        TestServer::new(build_router(state.clone())).expect("Failed to create test server");
    Some((state, server))
}

async fn delete_account(server: &TestServer, access_token: &str, csrf_token: &str) {
    let resp = server
        .delete("/api/v2/account/")
        .add_header(
            authorization_header_name(),
            super::bearer_header(access_token),
        )
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .await;

    assert_eq!(
        resp.status_code().as_u16(),
        204,
        "DELETE /api/v2/account failed: {}",
        resp.text()
    );
}

#[tokio::test]
#[serial]
async fn account_delete_sets_retention_hold_and_cleanup_hard_deletes_eligible_user() {
    let Some((state, server)) = build_account_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    sqlx::query(
        "INSERT INTO support_tickets (user_id, ticket_type, subject, description, priority)
         VALUES ($1, 'bug', 'Delete flow', 'Regression coverage', 'medium')",
    )
    .bind(user_id)
    .execute(state.db.pool())
    .await
    .expect("insert support ticket");

    sqlx::query(
        "INSERT INTO media_uploads (user_id, object_key, media_type, size_bytes)
         VALUES ($1, $2, 'clip', 1024)",
    )
    .bind(user_id)
    .bind(format!("media/clip/{user_id}-retention-test.bin"))
    .execute(state.db.pool())
    .await
    .expect("insert media upload");

    delete_account(&server, &access_token, &csrf_token).await;

    let deleted_user = sqlx::query(
        "SELECT deleted_at, deletion_hold_until, deletion_retention_basis, username, email, display_name, password_hash
         FROM users
         WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await
    .expect("load deleted user row");

    assert!(deleted_user
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
        .expect("deleted_at")
        .is_some());
    assert!(deleted_user
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deletion_hold_until")
        .expect("deletion_hold_until")
        .is_some());
    assert_eq!(
        deleted_user
            .try_get::<String, _>("deletion_retention_basis")
            .expect("deletion_retention_basis"),
        retention::DELETED_ACCOUNT_BASIS_PENDING
    );
    assert_eq!(
        deleted_user
            .try_get::<String, _>("display_name")
            .expect("display_name"),
        "[deleted user]"
    );
    assert!(deleted_user
        .try_get::<String, _>("username")
        .expect("username")
        .starts_with("[deleted_"));
    assert!(deleted_user
        .try_get::<String, _>("email")
        .expect("email")
        .starts_with("deleted+"));
    assert!(deleted_user
        .try_get::<Option<String>, _>("password_hash")
        .expect("password_hash")
        .is_none());

    let support_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM support_tickets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(state.db.pool())
            .await
            .expect("support count");
    let media_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_uploads WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(state.db.pool())
            .await
            .expect("media count");
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(state.db.pool())
        .await
        .expect("session count");

    assert_eq!(
        support_count, 0,
        "support tickets should be purged on delete"
    );
    assert_eq!(
        media_count, 0,
        "media upload rows should be purged on delete"
    );
    assert_eq!(session_count, 0, "sessions should be purged on delete");

    sqlx::query(
        "UPDATE users
         SET deleted_at = NOW() - INTERVAL '31 days',
             deletion_hold_until = NOW() - INTERVAL '1 day'
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(state.db.pool())
    .await
    .expect("age deleted user beyond hold");

    retention::run_cleanup(&state)
        .await
        .expect("run retention cleanup");

    let remaining_user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(state.db.pool())
        .await
        .expect("remaining user count");
    assert_eq!(
        remaining_user_count, 0,
        "eligible deleted user should be hard-deleted after hold"
    );
}

#[tokio::test]
#[serial]
async fn retention_cleanup_preserves_anonymized_shell_when_history_requires_it() {
    let Some((state, server)) = build_account_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    let conversation_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();

    sqlx::query("INSERT INTO dm_conversations (id) VALUES ($1)")
        .bind(conversation_id)
        .execute(state.db.pool())
        .await
        .expect("insert dm conversation");

    sqlx::query(
        "INSERT INTO messages (id, conversation_id, sender_id, ciphertext, message_type, delivered)
         VALUES ($1, $2, $3, $4, 'text', TRUE)",
    )
    .bind(message_id)
    .bind(conversation_id)
    .bind(user_id)
    .bind(vec![1_u8, 2, 3, 4])
    .execute(state.db.pool())
    .await
    .expect("insert historical message");

    delete_account(&server, &access_token, &csrf_token).await;

    sqlx::query(
        "UPDATE users
         SET deleted_at = NOW() - INTERVAL '31 days',
             deletion_hold_until = NOW() - INTERVAL '1 day'
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(state.db.pool())
    .await
    .expect("age deleted user beyond hold");

    retention::run_cleanup(&state)
        .await
        .expect("run retention cleanup");

    let retained_user = sqlx::query(
        "SELECT deleted_at, deletion_retention_basis, username
         FROM users
         WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await
    .expect("load retained deleted user");

    assert!(retained_user
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
        .expect("deleted_at")
        .is_some());
    assert_eq!(
        retained_user
            .try_get::<String, _>("deletion_retention_basis")
            .expect("deletion_retention_basis"),
        retention::DELETED_ACCOUNT_BASIS_REFERENTIAL_INTEGRITY
    );
    assert!(retained_user
        .try_get::<String, _>("username")
        .expect("username")
        .starts_with("[deleted_"));

    let message_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE id = $1")
        .bind(message_id)
        .fetch_one(state.db.pool())
        .await
        .expect("message count");
    assert_eq!(
        message_count, 1,
        "historical message should still exist when it anchors referential integrity"
    );
}
