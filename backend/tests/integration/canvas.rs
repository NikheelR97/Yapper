//! Integration tests for Canvas business rules.

use super::{register_test_session, spawn_test_server_with_pool, TestClient};
use axum_test::TestServer;
use sqlx::PgPool;
use uuid::Uuid;
use yapper_server::canvas::types::MAX_MUSIC_QUEUE_SIZE;

async fn build_canvas_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

async fn create_public_server(client: &TestClient<'_>, name: &str) -> Uuid {
    let resp = client
        .post("/api/v2/servers")
        .json(&serde_json::json!({
            "name": name,
            "description": "integration canvas test",
            "is_public": true,
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "server create failed: {} - {}",
        resp.status_code(),
        resp.text()
    );

    let body: serde_json::Value = resp.json();
    body["id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing server id")
}

async fn general_channel_id(state: &yapper_server::AppState, server_id: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM channels WHERE server_id = $1 AND name = 'general' LIMIT 1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await
        .expect("general channel id")
}

#[sqlx::test(migrations = "./migrations")]
async fn poll_votes_are_rejected_after_the_first_vote(pool: PgPool) {
    let Some((state, server)) = build_canvas_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let owner_session = register_test_session(&server, &format!("canvas_owner_{suffix}")).await;
    let owner_client = TestClient::from_session(&server, &owner_session);
    let server_id = create_public_server(&owner_client, &format!("Canvas {suffix}")).await;
    let channel_id = general_channel_id(&state, server_id).await;

    let create_poll = owner_client
        .post(&format!("/api/v2/canvas/channels/{channel_id}/polls"))
        .json(&serde_json::json!({
            "poll_type": "binary",
            "question": "Which option should win?",
            "options": ["Yes", "No"],
            "anonymous": false
        }))
        .await;

    assert!(
        create_poll.status_code().is_success(),
        "poll create failed: {} - {}",
        create_poll.status_code(),
        create_poll.text()
    );
    let poll_body: serde_json::Value = create_poll.json();
    let poll_id: Uuid = poll_body["id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing poll id");

    let vote_path = format!("/api/v2/canvas/polls/{poll_id}/vote");
    let first_vote = owner_client
        .post(&vote_path)
        .json(&serde_json::json!({ "option_index": 0 }))
        .await;
    assert!(
        first_vote.status_code().is_success(),
        "first vote failed: {} - {}",
        first_vote.status_code(),
        first_vote.text()
    );

    let second_vote = owner_client
        .post(&vote_path)
        .json(&serde_json::json!({ "option_index": 0 }))
        .await;
    assert_eq!(second_vote.status_code().as_u16(), 409);
    assert!(
        second_vote.text().contains("Already voted on this poll"),
        "unexpected duplicate-vote rejection: {}",
        second_vote.text()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn music_queue_rejects_non_admin_enqueue_attempts(pool: PgPool) {
    let Some((_state, server)) = build_canvas_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let owner_session = register_test_session(&server, &format!("canvas_owner_{suffix}")).await;
    let member_session = register_test_session(&server, &format!("canvas_member_{suffix}")).await;
    let owner_client = TestClient::from_session(&server, &owner_session);
    let member_client = TestClient::from_session(&server, &member_session);
    let server_id = create_public_server(&owner_client, &format!("Canvas Queue {suffix}")).await;

    let join = member_client
        .post(&format!("/api/v2/servers/{server_id}/join"))
        .await;
    assert!(join.status_code().is_success(), "member join failed: {}", join.text());

    let enqueue = member_client
        .post(&format!("/api/v2/canvas/servers/{server_id}/music/queue"))
        .json(&serde_json::json!({
            "artist": "Test Artist",
            "title": "Test Track",
            "duration_secs": 180,
            "source_url": null,
            "album_art_url": null
        }))
        .await;

    assert_eq!(enqueue.status_code().as_u16(), 403, "non-admin enqueue should be rejected");
}

#[sqlx::test(migrations = "./migrations")]
async fn music_queue_rejects_tracks_when_at_capacity(pool: PgPool) {
    let Some((state, server)) = build_canvas_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let owner_session = register_test_session(&server, &format!("canvas_queue_owner_{suffix}")).await;
    let owner_client = TestClient::from_session(&server, &owner_session);
    let owner_id = owner_session.user_id;
    let server_id = create_public_server(&owner_client, &format!("Canvas Cap {suffix}")).await;

    for position in 0..MAX_MUSIC_QUEUE_SIZE as i32 {
        sqlx::query(
            "INSERT INTO music_queue (server_id, added_by, artist, title, duration_secs, position) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(server_id)
        .bind(owner_id)
        .bind(format!("Artist {position}"))
        .bind(format!("Track {position}"))
        .bind(180_i32)
        .bind(position)
        .execute(state.db.pool())
        .await
        .expect("seed music queue entry");
    }

    let response = owner_client
        .post(&format!("/api/v2/canvas/servers/{server_id}/music/queue"))
        .json(&serde_json::json!({
            "artist": "Overflow Artist",
            "title": "Overflow Track",
            "duration_secs": 180
        }))
        .await;

    assert_eq!(response.status_code().as_u16(), 409);
    assert!(
        response.text().contains("Music queue is full"),
        "unexpected queue-cap rejection: {}",
        response.text()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn clip_reactions_are_idempotent_for_the_same_user_and_emoji(pool: PgPool) {
    let Some((state, server)) = build_canvas_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let owner_session = register_test_session(&server, &format!("canvas_clip_owner_{suffix}")).await;
    let owner_client = TestClient::from_session(&server, &owner_session);
    let owner_id = owner_session.user_id;
    let owner_device_id = owner_session.device_id.expect("missing owner device id");
    let server_id = create_public_server(&owner_client, &format!("Canvas Clips {suffix}")).await;
    let channel_id = general_channel_id(&state, server_id).await;
    let clip_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, sender_device_id, ciphertext, message_type, delivered) \
         VALUES ($1, $2, $3, $4, $5, 'clip', TRUE)",
    )
    .bind(clip_id)
    .bind(channel_id)
    .bind(owner_id)
    .bind(owner_device_id)
    .bind(vec![1_u8, 2, 3, 4])
    .execute(state.db.pool())
    .await
    .expect("seed clip message");

    let reaction_path = format!("/api/v2/canvas/clips/{clip_id}/reactions");
    for _ in 0..2 {
        let response = owner_client
            .put(&reaction_path)
            .json(&serde_json::json!({ "emoji": ":fire:" }))
            .await;
        assert_eq!(response.status_code().as_u16(), 204, "reaction add failed: {}", response.text());
    }

    let reaction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM clip_reactions WHERE clip_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind(clip_id)
    .bind(owner_id)
    .bind(":fire:")
    .fetch_one(state.db.pool())
    .await
    .expect("reaction count");

    assert_eq!(reaction_count, 1, "duplicate reactions should be stored once");
}
