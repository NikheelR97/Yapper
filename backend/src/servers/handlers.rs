use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};
use super::service::{self, CreateInviteInput, UpdateServerInput};

// ─── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateServerReq {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateServerReq {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub icon_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInviteReq {
    pub max_uses: Option<i32>,
    /// Hours until expiry. None = never expires.
    pub expires_in_hours: Option<i64>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

pub async fn create_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateServerReq>,
) -> AppResult<(StatusCode, Json<service::ServerResp>)> {
    let name = req.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest("Server name must be 1–100 characters".into()));
    }
    let resp = service::create_server(
        auth.user_id,
        name,
        req.description,
        req.is_public.unwrap_or(true),
        &state,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn list_my_servers(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<service::ServerResp>>> {
    let servers = service::list_my_servers(auth.user_id, &state).await?;
    Ok(Json(servers))
}

pub async fn get_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<service::ServerResp>> {
    let server = service::get_server(auth.user_id, server_id, &state).await?;
    Ok(Json(server))
}

pub async fn update_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<UpdateServerReq>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(ref name) = req.name {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(AppError::BadRequest("Name must be 1–100 characters".into()));
        }
    }
    service::update_server(
        auth.user_id,
        server_id,
        UpdateServerInput {
            name: req.name,
            description: req.description,
            is_public: req.is_public,
            icon_url: req.icon_url,
        },
        &state,
    )
    .await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub async fn join_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = service::join_server_public(auth.user_id, server_id, &state).await?;
    Ok(Json(result))
}

pub async fn leave_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    service::leave_server(auth.user_id, server_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<CreateInviteReq>,
) -> AppResult<(StatusCode, Json<service::InviteResp>)> {
    let resp = service::create_invite(
        auth.user_id,
        server_id,
        CreateInviteInput {
            max_uses: req.max_uses,
            expires_in_hours: req.expires_in_hours,
        },
        &state,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn join_by_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = service::join_by_invite(auth.user_id, code, &state).await?;
    Ok(Json(result))
}
