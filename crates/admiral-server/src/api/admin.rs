//! Admin API: invites (create, list), user extend. Requires role=admin.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use crate::entities::{entitlement, invite, user};
use crate::error::{AppError, Result};
use crate::state::AppState;
use sea_orm::ActiveValue::Set;

async fn require_admin(state: &AppState, user_id: Uuid) -> Result<()> {
    let u = user::Entity::find_by_id(user_id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("user".into()))?;
    if u.role != "admin" {
        return Err(AppError::Forbidden("admin only".into()));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    pub trial_days: Option<i32>,
    pub max_uses: Option<i32>,
    pub expires_at: Option<String>,
}

#[derive(serde::Serialize)]
pub struct InviteResponse {
    pub id: Uuid,
    pub code: String,
    pub trial_days: i32,
    pub max_uses: i32,
    pub used_count: i32,
    pub expires_at: Option<String>,
    pub created_at: String,
}

pub async fn create_invite(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>)> {
    require_admin(&state, auth_user.0).await?;
    let code = format!("INV-{}", Uuid::new_v4().to_string().replace('-', "").chars().take(8).collect::<String>().to_uppercase());
    let trial_days = req.trial_days.unwrap_or(7);
    let max_uses = req.max_uses.unwrap_or(1);
    let expires_at = req.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|t| t.with_timezone(&chrono::Utc));
    let now = chrono::Utc::now();
    let active = invite::ActiveModel {
        id: Set(Uuid::new_v4()),
        code: Set(code.clone()),
        trial_days: Set(trial_days),
        max_uses: Set(max_uses),
        used_count: Set(0),
        created_by: Set(Some(auth_user.0)),
        expires_at: Set(expires_at.map(Into::into)),
        created_at: Set(now.into()),
        ..Default::default()
    };
    let model = active.insert(&state.db).await?;
    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            id: model.id,
            code: model.code,
            trial_days: model.trial_days,
            max_uses: model.max_uses,
            used_count: model.used_count,
            expires_at: model.expires_at.map(|t| t.to_rfc3339()),
            created_at: model.created_at.to_rfc3339(),
        }),
    ))
}

pub async fn list_invites(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<Vec<InviteResponse>>> {
    require_admin(&state, auth_user.0).await?;
    let rows = invite::Entity::find()
        .order_by_desc(invite::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let list = rows
        .into_iter()
        .map(|r| InviteResponse {
            id: r.id,
            code: r.code,
            trial_days: r.trial_days,
            max_uses: r.max_uses,
            used_count: r.used_count,
            expires_at: r.expires_at.map(|t| t.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct ExtendUserRequest {
    pub days: i64,
    pub reason: Option<String>,
}

pub async fn extend_user(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ExtendUserRequest>,
) -> Result<StatusCode> {
    require_admin(&state, auth_user.0).await?;
    let ent = entitlement::Entity::find()
        .filter(entitlement::Column::UserId.eq(id))
        .filter(entitlement::Column::IsActive.eq(true))
        .order_by_desc(entitlement::Column::ExpiresAt)
        .one(&state.db)
        .await?;
    let Some(e) = ent else {
        return Err(AppError::NotFound("no active entitlement".into()));
    };
    let exp_dt = e.expires_at.with_timezone(&chrono::Utc);
    let new_expires = exp_dt + chrono::Duration::days(req.days);
    let mut active: entitlement::ActiveModel = e.into();
    active.expires_at = Set(new_expires.into());
    active.extended_by = Set(Some(auth_user.0));
    active.extend_reason = Set(req.reason);
    active.save(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
