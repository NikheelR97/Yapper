use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    channels::service::{require_admin, require_member},
    error::{AppError, AppResult},
    AppState,
};

/// Max parents that can be notified for a single parental intercept event.
const MAX_PARENTS_PER_CHILD: usize = 10;

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServerResp {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub icon_url: Option<String>,
    pub description: Option<String>,
    pub is_public: bool,
    pub member_count: i64,
    pub role: Option<String>,
}

#[derive(Serialize)]
pub struct InviteResp {
    pub code: String,
    pub server_id: Uuid,
    pub max_uses: Option<i32>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

// ─── Input types ──────────────────────────────────────────────────────────────

pub struct UpdateServerInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub icon_url: Option<String>,
}

pub struct CreateInviteInput {
    pub max_uses: Option<i32>,
    /// Hours until expiry. None = never expires.
    pub expires_in_hours: Option<i64>,
}

// ─── Service functions ────────────────────────────────────────────────────────

pub async fn create_server(
    user_id: Uuid,
    name: String,
    description: Option<String>,
    is_public: bool,
    state: &AppState,
) -> AppResult<ServerResp> {
    debug_assert!(
        !name.is_empty(),
        "name must be non-empty (validated by handler)"
    );
    debug_assert!(
        name.len() <= 100,
        "name must be ≤100 chars (validated by handler)"
    );

    let base_slug = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let base_slug = base_slug.trim_matches('-').to_string();
    let slug = format!("{}-{}", base_slug, &Uuid::new_v4().to_string()[..8]);

    let mut tx = state.db.pool().begin().await?;

    let server_row = sqlx::query(
        "INSERT INTO servers (name, slug, owner_id, description, is_public) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, name, slug, owner_id, icon_url, description, is_public",
    )
    .bind(&name)
    .bind(&slug)
    .bind(user_id)
    .bind(&description)
    .bind(is_public)
    .fetch_one(&mut *tx)
    .await?;

    let server_id: Uuid = server_row.try_get("id")?;

    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(user_id)
    .bind(server_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO channels (server_id, name, type, position) VALUES ($1, 'general', 'text', 0)",
    )
    .bind(server_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    debug_assert!(
        server_id != Uuid::nil(),
        "server_id must be valid after insert"
    );

    Ok(ServerResp {
        id: server_id,
        name: server_row.try_get("name")?,
        slug: server_row.try_get("slug")?,
        owner_id: server_row.try_get("owner_id")?,
        icon_url: server_row.try_get("icon_url")?,
        description: server_row.try_get("description")?,
        is_public: server_row.try_get("is_public")?,
        member_count: 1,
        role: Some("owner".to_string()),
    })
}

pub async fn list_my_servers(user_id: Uuid, state: &AppState) -> AppResult<Vec<ServerResp>> {
    debug_assert!(user_id != Uuid::nil());

    let rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.owner_id, s.icon_url, s.description, s.is_public, \
                sm.role, \
                (SELECT COUNT(*) FROM server_memberships WHERE server_id = s.id) AS member_count \
         FROM servers s \
         JOIN server_memberships sm ON sm.server_id = s.id AND sm.user_id = $1 \
         ORDER BY sm.joined_at ASC",
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| {
            Ok(ServerResp {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                slug: r.try_get("slug")?,
                owner_id: r.try_get("owner_id")?,
                icon_url: r.try_get("icon_url")?,
                description: r.try_get("description")?,
                is_public: r.try_get("is_public")?,
                member_count: r.try_get("member_count")?,
                role: r.try_get("role")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)
}

pub async fn get_server(user_id: Uuid, server_id: Uuid, state: &AppState) -> AppResult<ServerResp> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let row = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.owner_id, s.icon_url, s.description, s.is_public, \
                sm.role, \
                (SELECT COUNT(*) FROM server_memberships WHERE server_id = s.id) AS member_count \
         FROM servers s \
         LEFT JOIN server_memberships sm ON sm.server_id = s.id AND sm.user_id = $2 \
         WHERE s.id = $1",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    let is_public: bool = row.try_get("is_public")?;
    let role: Option<String> = row.try_get("role")?;

    if !is_public && role.is_none() {
        return Err(AppError::Forbidden);
    }

    Ok(ServerResp {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        owner_id: row.try_get("owner_id")?,
        icon_url: row.try_get("icon_url")?,
        description: row.try_get("description")?,
        is_public,
        member_count: row.try_get("member_count")?,
        role,
    })
}

pub async fn update_server(
    user_id: Uuid,
    server_id: Uuid,
    input: UpdateServerInput,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    require_admin(state, user_id, server_id).await?;

    sqlx::query(
        "UPDATE servers SET \
            name        = COALESCE($2, name), \
            description = COALESCE($3, description), \
            is_public   = COALESCE($4, is_public), \
            icon_url    = COALESCE($5, icon_url) \
         WHERE id = $1",
    )
    .bind(server_id)
    .bind(input.name.as_deref())
    .bind(input.description.as_deref())
    .bind(input.is_public)
    .bind(input.icon_url.as_deref())
    .execute(state.db.pool())
    .await?;

    Ok(())
}

pub async fn join_server_public(
    user_id: Uuid,
    server_id: Uuid,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let server_row = sqlx::query("SELECT is_public FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    let is_public: bool = server_row.try_get("is_public")?;
    if !is_public {
        return Err(AppError::Forbidden);
    }

    do_join(user_id, server_id, None, state).await
}

pub async fn leave_server(user_id: Uuid, server_id: Uuid, state: &AppState) -> AppResult<()> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let row =
        sqlx::query("SELECT role FROM server_memberships WHERE user_id = $1 AND server_id = $2")
            .bind(user_id)
            .bind(server_id)
            .fetch_optional(state.db.pool())
            .await?;

    match row {
        None => return Err(AppError::NotFound("Not a member".into())),
        Some(r) => {
            let role: String = r.try_get("role")?;
            if role == "owner" {
                return Err(AppError::BadRequest(
                    "Owner cannot leave — transfer ownership first".into(),
                ));
            }
        }
    }

    sqlx::query("DELETE FROM server_memberships WHERE user_id = $1 AND server_id = $2")
        .bind(user_id)
        .bind(server_id)
        .execute(state.db.pool())
        .await?;

    Ok(())
}

pub async fn create_invite(
    user_id: Uuid,
    server_id: Uuid,
    input: CreateInviteInput,
    state: &AppState,
) -> AppResult<InviteResp> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    require_member(state, user_id, server_id).await?;

    let code = generate_invite_code();
    let expires_at = input
        .expires_in_hours
        .map(|h| Utc::now() + chrono::Duration::hours(h));

    sqlx::query(
        "INSERT INTO server_invite_links (server_id, code, creator_id, max_uses, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(server_id)
    .bind(&code)
    .bind(user_id)
    .bind(input.max_uses)
    .bind(expires_at)
    .execute(state.db.pool())
    .await?;

    Ok(InviteResp {
        code,
        server_id,
        max_uses: input.max_uses,
        expires_at,
    })
}

pub async fn join_by_invite(
    user_id: Uuid,
    code: String,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(!code.is_empty(), "invite code must be non-empty");

    let invite_row = sqlx::query(
        "SELECT i.id, i.server_id, i.uses, i.max_uses, i.expires_at \
         FROM server_invite_links i \
         WHERE i.code = $1",
    )
    .bind(&code)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Invalid invite code".into()))?;

    let expires_at: Option<chrono::DateTime<Utc>> = invite_row.try_get("expires_at")?;
    if let Some(exp) = expires_at {
        if Utc::now() > exp {
            return Err(AppError::BadRequest("Invite link has expired".into()));
        }
    }

    let uses: i32 = invite_row.try_get("uses")?;
    let max_uses: Option<i32> = invite_row.try_get("max_uses")?;
    if let Some(max) = max_uses {
        if uses >= max {
            return Err(AppError::BadRequest(
                "Invite link has reached its use limit".into(),
            ));
        }
    }

    let server_id: Uuid = invite_row.try_get("server_id")?;
    let invite_id: Uuid = invite_row.try_get("id")?;

    sqlx::query("UPDATE server_invite_links SET uses = uses + 1 WHERE id = $1")
        .bind(invite_id)
        .execute(state.db.pool())
        .await?;

    do_join(user_id, server_id, Some(code), state).await
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

pub(crate) async fn do_join(
    user_id: Uuid,
    server_id: Uuid,
    invite_code: Option<String>,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    if user_id == Uuid::nil() || server_id == Uuid::nil() {
        return Err(AppError::BadRequest("Invalid IDs".into()));
    }

    let mut tx = state.db.pool().begin().await?;

    let already =
        sqlx::query("SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2")
            .bind(user_id)
            .bind(server_id)
            .fetch_optional(&mut *tx)
            .await?;

    if already.is_some() {
        tx.rollback().await.ok();
        return Ok(serde_json::json!({ "status": "already_member" }));
    }

    let user_row =
        sqlx::query("SELECT parental_controls_enabled FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    let parental_controls: bool = user_row.try_get("parental_controls_enabled")?;
    if parental_controls {
        tx.rollback().await.ok();
        return handle_parental_intercept(user_id, server_id, invite_code, state).await;
    }

    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role) VALUES ($1, $2, 'member') \
         ON CONFLICT (user_id, server_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(server_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(serde_json::json!({ "status": "joined" }))
}

async fn handle_parental_intercept(
    user_id: Uuid,
    server_id: Uuid,
    invite_code: Option<String>,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    let server_row = sqlx::query("SELECT name FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await?;
    let server_name: String = server_row.try_get("name")?;

    let pending_id: Uuid = sqlx::query(
        "INSERT INTO pending_server_joins (child_user_id, server_id, server_name, invite_code) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(invite_code.as_deref())
    .fetch_one(state.db.pool())
    .await?
    .try_get("id")?;

    notify_parents(user_id, server_id, &server_name, pending_id, state).await?;

    Ok(serde_json::json!({
        "status": "pending_approval",
        "message": "A parent must approve this server join"
    }))
}

async fn notify_parents(
    user_id: Uuid,
    server_id: Uuid,
    server_name: &str,
    pending_id: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(pending_id != Uuid::nil());

    let parents = sqlx::query(
        "SELECT parent_user_id FROM parent_child_relationships \
         WHERE child_user_id = $1 \
         LIMIT $2",
    )
    .bind(user_id)
    .bind(MAX_PARENTS_PER_CHILD as i64)
    .fetch_all(state.db.pool())
    .await?;

    for p in parents.iter().take(MAX_PARENTS_PER_CHILD) {
        let parent_id: Uuid = p.try_get("parent_user_id")?;

        sqlx::query(
            "INSERT INTO parent_notifications (parent_user_id, child_user_id, type, reference_id) \
             VALUES ($1, $2, 'server_join', $3)",
        )
        .bind(parent_id)
        .bind(user_id)
        .bind(pending_id)
        .execute(state.db.pool())
        .await?;

        state.hub.send_to_user(
            &parent_id,
            crate::hub::WsOutbound::ParentNotification {
                payload: serde_json::json!({
                    "type": "server_join",
                    "pending_id": pending_id,
                    "child_id": user_id,
                    "server_id": server_id,
                    "server_name": server_name,
                }),
            },
        );
    }

    Ok(())
}

pub fn generate_invite_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let uid = Uuid::new_v4();
    let base = uid.simple().to_string();
    format!("{}{:x}", &base[..7], ts & 0xF)
}
