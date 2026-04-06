pub mod handlers;
pub mod service;
pub mod types;

use axum::{
    extract::DefaultBodyLimit,
    routing::get,
    Router,
};

use crate::AppState;
use handlers::*;

pub fn v2_router() -> Router<AppState> {
    let message_routes = Router::new()
        .route("/:id/messages", get(list_messages_v2).post(send_message_v2))
        .layer(DefaultBodyLimit::max(
            crate::constants::MAX_MESSAGE_REQUEST_BODY_SIZE,
        ));

    Router::new()
        .route(
            "/",
            axum::routing::post(create_or_get_conversation_v2).get(list_conversations_v2),
        )
        .merge(message_routes)
}

#[cfg(test)]
mod schema_invariant_tests {
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn insert_channel_context(pool: &PgPool) -> (Uuid, Uuid) {
        let user_id = Uuid::new_v4();
        let server_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO users (id, email, username, display_name, password_hash, gdpr_consent_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(user_id)
        .bind(format!("msg_schema_{user_id}@integration.test"))
        .bind(format!("msg_schema_{user_id}"))
        .bind("Message Schema User")
        .bind("hash")
        .execute(pool)
        .await
        .expect("insert user");

        sqlx::query(
            "INSERT INTO servers (id, name, slug, owner_id, member_count)
             VALUES ($1, $2, $3, $4, 1)",
        )
        .bind(server_id)
        .bind("Schema Test Server")
        .bind(format!(
            "schema-test-{}",
            &server_id.simple().to_string()[..8]
        ))
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert server");

        sqlx::query(
            "INSERT INTO channels (id, server_id, name, type, position)
             VALUES ($1, $2, 'general', 'text', 0)",
        )
        .bind(channel_id)
        .bind(server_id)
        .execute(pool)
        .await
        .expect("insert channel");

        (user_id, channel_id)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_message_rejects_both_ciphertext_and_plaintext(pool: PgPool) {
        let (user_id, channel_id) = insert_channel_context(&pool).await;

        let err = sqlx::query(
            "INSERT INTO messages (id, channel_id, sender_id, ciphertext, plaintext, message_type, delivered)
             VALUES ($1, $2, $3, $4, $5, 'text', FALSE)",
        )
        .bind(Uuid::new_v4())
        .bind(channel_id)
        .bind(user_id)
        .bind(vec![1_u8, 2_u8, 3_u8])
        .bind("plaintext should not coexist")
        .execute(&pool)
        .await
        .expect_err("both ciphertext and plaintext should violate the constraint");

        let db_err = err
            .as_database_error()
            .expect("constraint violation should surface as a database error");
        assert_eq!(
            db_err.constraint(),
            Some("messages_ciphertext_xor_plaintext"),
            "unexpected constraint name: {db_err:?}"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_message_rejects_neither_ciphertext_nor_plaintext(pool: PgPool) {
        let (user_id, channel_id) = insert_channel_context(&pool).await;

        let err = sqlx::query(
            "INSERT INTO messages (id, channel_id, sender_id, message_type, delivered)
             VALUES ($1, $2, $3, 'text', FALSE)",
        )
        .bind(Uuid::new_v4())
        .bind(channel_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect_err(
            "channel rows with neither ciphertext nor plaintext should violate the constraint",
        );

        let db_err = err
            .as_database_error()
            .expect("constraint violation should surface as a database error");
        assert_eq!(
            db_err.constraint(),
            Some("messages_ciphertext_xor_plaintext"),
            "unexpected constraint name: {db_err:?}"
        );
    }
}
