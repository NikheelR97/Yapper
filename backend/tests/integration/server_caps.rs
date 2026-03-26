//! Integration tests for transactional server member-cap enforcement.

use super::{
    authorization_header_name, bearer_header, build_test_state, csrf_header_name,
    csrf_header_value, create_test_user_with_device, test_device_bootstrap,
};
use axum_test::TestServer;
use serial_test::serial;
use uuid::Uuid;
use yapper_server::build_router;

async fn build_server_caps_test_server() -> Option<(yapper_server::AppState, TestServer)> {
    let state = build_test_state().await?;
    let server = TestServer::new(build_router(state.clone())).expect("Failed to create test server");
    Some((state, server))
}

async fn login_v2_user(
    server: &TestServer,
    email: &str,
    password: &str,
) -> serde_json::Value {
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let resp = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "device": test_device_bootstrap(&suffix),
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "v2 login failed: {} - {}",
        resp.status_code(),
        resp.text()
    );

    resp.json()
}

async fn create_public_server(
    server: &TestServer,
    access_token: &str,
    csrf_token: &str,
    name: &str,
) -> Uuid {
    let resp = server
        .post("/api/v2/servers/")
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .json(&serde_json::json!({
            "name": name,
            "description": "integration cap test",
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

async fn create_invite(
    server: &TestServer,
    access_token: &str,
    csrf_token: &str,
    server_id: Uuid,
    max_uses: i32,
) -> String {
    let path = format!("/api/v2/servers/{server_id}/invite");
    let response = server
        .post(&path)
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .json(&serde_json::json!({
            "max_uses": max_uses,
            "expires_in_hours": 24,
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "invite create failed: {} - {}",
        response.status_code(),
        response.text()
    );

    let body: serde_json::Value = response.json();
    body["code"]
        .as_str()
        .expect("missing invite code")
        .to_string()
}

async fn cleanup_user_prefix(state: &yapper_server::AppState, prefix: &str) {
    let pattern = format!("{prefix}%");
    let _ = sqlx::query("DELETE FROM users WHERE email LIKE $1")
        .bind(pattern)
        .execute(state.db.pool())
        .await;
}

#[tokio::test]
#[serial]
async fn server_member_cap_is_atomic_under_concurrent_joins() {
    let Some((state, server)) = build_server_caps_test_server().await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_, owner_access, owner_csrf, _) = create_test_user_with_device(&server, &format!("owner_{suffix}")).await;
    let server_id = create_public_server(&server, &owner_access, &owner_csrf, &format!("Cap Test {suffix}")).await;

    let marker = server_id.to_string().replace('-', "");
    let dummy_prefix = format!("cap_{marker}_");
    sqlx::query(
        "INSERT INTO users (id, email, username, display_name, gdpr_consent_at)
         SELECT gen_random_uuid(),
                $1 || gs::text || '@integration.test',
                $2 || gs::text,
                $2 || gs::text,
                NOW()
         FROM generate_series(1, $3) AS gs",
    )
    .bind(&dummy_prefix)
    .bind(&dummy_prefix)
    .bind(498_i64)
    .execute(state.db.pool())
    .await
    .expect("seed dummy users");

    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role)
         SELECT id, $1, 'member'
         FROM users
         WHERE email LIKE $2",
    )
    .bind(server_id)
    .bind(format!("{dummy_prefix}%@integration.test"))
    .execute(state.db.pool())
    .await
    .expect("seed dummy memberships");

    let (_, joiner_a_access, joiner_a_csrf, _) =
        create_test_user_with_device(&server, &format!("joiner_a_{suffix}")).await;
    let (_, joiner_b_access, joiner_b_csrf, _) =
        create_test_user_with_device(&server, &format!("joiner_b_{suffix}")).await;
    let join_path = format!("/api/v2/servers/{server_id}/join");

    let join_a = async {
        server
            .post(&join_path)
            .add_header(authorization_header_name(), bearer_header(&joiner_a_access))
            .add_header(csrf_header_name(), csrf_header_value(&joiner_a_csrf))
            .await
    };
    let join_b = async {
        server
            .post(&join_path)
            .add_header(authorization_header_name(), bearer_header(&joiner_b_access))
            .add_header(csrf_header_name(), csrf_header_value(&joiner_b_csrf))
            .await
    };

    let (resp_a, resp_b) = tokio::join!(join_a, join_b);
    let statuses = [resp_a.status_code().as_u16(), resp_b.status_code().as_u16()];
    assert!(statuses.contains(&200), "one join should succeed: {:?}", statuses);
    assert!(statuses.contains(&403), "one join should be rejected at cap: {:?}", statuses);

    let member_count: i64 = sqlx::query_scalar("SELECT member_count FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await
        .expect("server member_count");
    assert_eq!(member_count, 500);

    let _ = sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(server_id)
        .execute(state.db.pool())
        .await;
    cleanup_user_prefix(&state, &dummy_prefix).await;
    cleanup_user_prefix(&state, &format!("test_owner_{suffix}")).await;
    cleanup_user_prefix(&state, &format!("test_joiner_a_{suffix}")).await;
    cleanup_user_prefix(&state, &format!("test_joiner_b_{suffix}")).await;
}

#[tokio::test]
#[serial]
async fn parental_server_join_respects_cap_and_rolls_back_approval() {
    let Some((state, server)) = build_server_caps_test_server().await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_, parent_access, parent_csrf, _) = create_test_user_with_device(&server, &format!("parent_{suffix}")).await;

    let child_email = format!("MiXeDChild_{suffix}@Integration.Test");
    let child_password = format!("ChildPass123!{suffix}");
    let child_resp = server
        .post("/api/v2/parental/children")
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
        .json(&serde_json::json!({
            "username": format!("child_{suffix}"),
            "display_name": "Parented Child",
            "email": child_email,
            "password": child_password,
            "date_of_birth": "2015-03-26",
        }))
        .await;

    assert!(
        child_resp.status_code().is_success(),
        "child creation failed: {} - {}",
        child_resp.status_code(),
        child_resp.text()
    );
    let child_body: serde_json::Value = child_resp.json();
    let child_id: Uuid = child_body["child_id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing child id");

    let child_login = login_v2_user(&server, &child_email, &child_password).await;
    let child_access = child_login["access_token"]
        .as_str()
        .expect("missing child access token")
        .to_string();
    let child_csrf = child_login["csrf_token"]
        .as_str()
        .expect("missing child csrf token")
        .to_string();

    let server_id = create_public_server(&server, &parent_access, &parent_csrf, &format!("Parental Cap {suffix}")).await;

    let marker = server_id.to_string().replace('-', "");
    let dummy_prefix = format!("parent_cap_{marker}_");
    sqlx::query(
        "INSERT INTO users (id, email, username, display_name, gdpr_consent_at)
         SELECT gen_random_uuid(),
                $1 || gs::text || '@integration.test',
                $2 || gs::text,
                $2 || gs::text,
                NOW()
         FROM generate_series(1, $3) AS gs",
    )
    .bind(&dummy_prefix)
    .bind(&dummy_prefix)
    .bind(499_i64)
    .execute(state.db.pool())
    .await
    .expect("seed dummy users");

    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role)
         SELECT id, $1, 'member'
         FROM users
         WHERE email LIKE $2",
    )
    .bind(server_id)
    .bind(format!("{dummy_prefix}%@integration.test"))
    .execute(state.db.pool())
    .await
    .expect("seed dummy memberships");

    let join_path = format!("/api/v2/servers/{server_id}/join");
    let join = server
        .post(&join_path)
        .add_header(authorization_header_name(), bearer_header(&child_access))
        .add_header(csrf_header_name(), csrf_header_value(&child_csrf))
        .await;
    assert!(join.status_code().is_success(), "child join should create a pending request: {}", join.text());
    let join_body: serde_json::Value = join.json();
    assert_eq!(join_body["status"], "pending_approval");

    let pending_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM pending_server_joins WHERE child_user_id = $1 AND server_id = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(child_id)
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await
    .expect("load pending server join");

    let approve_path = format!("/api/v2/parental/server-joins/{pending_id}/approve");
    let approve = server
        .patch(&approve_path)
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
        .await;
    assert_eq!(approve.status_code().as_u16(), 403, "approval should fail at cap: {}", approve.text());

    let pending_status: String = sqlx::query_scalar(
        "SELECT status FROM pending_server_joins WHERE id = $1",
    )
    .bind(pending_id)
    .fetch_one(state.db.pool())
    .await
    .expect("load pending status");
    assert_eq!(pending_status, "pending");

    let member_count: i64 = sqlx::query_scalar("SELECT member_count FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await
        .expect("server member_count");
    assert_eq!(member_count, 500);

    let _ = sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(server_id)
        .execute(state.db.pool())
        .await;
    cleanup_user_prefix(&state, &dummy_prefix).await;
    cleanup_user_prefix(&state, &format!("test_parent_{suffix}")).await;
    cleanup_user_prefix(&state, &format!("test_joiner_{suffix}")).await;
    cleanup_user_prefix(&state, &format!("mixedchild_{suffix}")).await;
}

#[tokio::test]
#[serial]
async fn parental_server_join_requests_are_deduplicated() {
    let Some((state, server)) = build_server_caps_test_server().await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_, parent_access, parent_csrf, _) =
        create_test_user_with_device(&server, &format!("parent_dedup_{suffix}")).await;

    let child_username = format!("child_dedup_{suffix}");
    let child_email = format!("child_dedup_{suffix}@integration.test");
    let child_password = format!("ChildPass123!{suffix}");
    let child_resp = server
        .post("/api/v2/parental/children")
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
        .json(&serde_json::json!({
            "username": &child_username,
            "display_name": "Pending Join Child",
            "email": &child_email,
            "password": &child_password,
            "date_of_birth": "2015-03-26",
        }))
        .await;
    assert!(child_resp.status_code().is_success(), "child creation failed: {}", child_resp.text());
    let child_body: serde_json::Value = child_resp.json();
    let child_id: Uuid = child_body["child_id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing child id");

    let child_login = login_v2_user(&server, &child_email, &child_password).await;
    let child_access = child_login["access_token"].as_str().expect("missing access token");
    let child_csrf = child_login["csrf_token"].as_str().expect("missing csrf token");

    let server_id = create_public_server(
        &server,
        &parent_access,
        &parent_csrf,
        &format!("Pending Join Dedup {suffix}"),
    )
    .await;
    let join_path = format!("/api/v2/servers/{server_id}/join");

    for _ in 0..2 {
        let response = server
            .post(&join_path)
            .add_header(authorization_header_name(), bearer_header(child_access))
            .add_header(csrf_header_name(), csrf_header_value(child_csrf))
            .await;
        assert!(response.status_code().is_success(), "join intercept failed: {}", response.text());
    }

    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pending_server_joins \
         WHERE child_user_id = $1 AND server_id = $2 AND status = 'pending'",
    )
    .bind(child_id)
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await
    .expect("pending join count");
    assert_eq!(pending_count, 1);
}

#[tokio::test]
#[serial]
async fn invite_use_is_consumed_only_after_approved_membership() {
    let Some((state, server)) = build_server_caps_test_server().await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_, parent_access, parent_csrf, _) =
        create_test_user_with_device(&server, &format!("invite_parent_{suffix}")).await;

    let child_email = format!("invite_child_{suffix}@integration.test");
    let child_password = format!("ChildPass123!{suffix}");
    let child_resp = server
        .post("/api/v2/parental/children")
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
        .json(&serde_json::json!({
            "username": format!("invite_child_{suffix}"),
            "display_name": "Invite Child",
            "email": &child_email,
            "password": &child_password,
            "date_of_birth": "2015-03-26",
        }))
        .await;
    assert!(child_resp.status_code().is_success(), "child creation failed: {}", child_resp.text());
    let child_body: serde_json::Value = child_resp.json();
    let child_id: Uuid = child_body["child_id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing child id");

    let child_login = login_v2_user(&server, &child_email, &child_password).await;
    let child_access = child_login["access_token"].as_str().expect("missing access token");
    let child_csrf = child_login["csrf_token"].as_str().expect("missing csrf token");

    let server_id = create_public_server(
        &server,
        &parent_access,
        &parent_csrf,
        &format!("Invite Approval {suffix}"),
    )
    .await;
    let invite_code = create_invite(&server, &parent_access, &parent_csrf, server_id, 1).await;

    let join_path = format!("/api/v2/servers/join/{invite_code}");
    let join = server
        .post(&join_path)
        .add_header(authorization_header_name(), bearer_header(child_access))
        .add_header(csrf_header_name(), csrf_header_value(child_csrf))
        .await;
    assert!(join.status_code().is_success(), "invite join failed: {}", join.text());
    let join_body: serde_json::Value = join.json();
    assert_eq!(join_body["status"], "pending_approval");

    let uses_before: i32 = sqlx::query_scalar(
        "SELECT uses FROM server_invite_links WHERE code = $1",
    )
    .bind(&invite_code)
    .fetch_one(state.db.pool())
    .await
    .expect("invite uses before approval");
    assert_eq!(uses_before, 0, "pending approval should not consume invite use");

    let pending_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM pending_server_joins \
         WHERE child_user_id = $1 AND server_id = $2 AND status = 'pending' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(child_id)
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await
    .expect("pending join id");

    let approve_path = format!("/api/v2/parental/server-joins/{pending_id}/approve");
    let approve = server
        .patch(&approve_path)
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
        .await;
    assert_eq!(approve.status_code().as_u16(), 204, "approval failed: {}", approve.text());

    let uses_after: i32 = sqlx::query_scalar(
        "SELECT uses FROM server_invite_links WHERE code = $1",
    )
    .bind(&invite_code)
    .fetch_one(state.db.pool())
    .await
    .expect("invite uses after approval");
    assert_eq!(uses_after, 1, "successful approval should consume invite use exactly once");
}
