use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    auth::{hash_password, issue_token, verify_password},
    db::entities::{device, user},
    state::SharedState,
};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub device_name: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub device_id: String,
    pub user_id: String,
}

pub async fn signup(
    State(state): State<SharedState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let exists = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some();

    if exists {
        return Err((StatusCode::CONFLICT, "username already taken".into()));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    let now = Utc::now();

    user::ActiveModel {
        id: Set(user_id),
        username: Set(req.username),
        display_name: Set(req.display_name),
        personal_message: Set(String::new()),
        avatar_url: Set(String::new()),
        password_hash: Set(password_hash),
        created_at: Set(now),
        deleted_at: Set(None),
    }.insert(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    device::ActiveModel {
        id: Set(device_id),
        user_id: Set(user_id),
        name: Set(req.device_name),
        platform: Set("desktop".into()),
        last_seen_at: Set(now),
    }.insert(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = issue_token(user_id, device_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token, device_id: device_id.to_string(), user_id: user_id.to_string() }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_name: String,
}

pub async fn login(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let user = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .filter(user::Column::DeletedAt.is_null())
        .one(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    if !verify_password(&req.password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    }

    let device_id = Uuid::new_v4();
    device::ActiveModel {
        id: Set(device_id),
        user_id: Set(user.id),
        name: Set(req.device_name),
        platform: Set("desktop".into()),
        last_seen_at: Set(Utc::now()),
    }.insert(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = issue_token(user.id, device_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token, device_id: device_id.to_string(), user_id: user.id.to_string() }))
}
