use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::api::{SharedState, UserId};
use crate::models::*;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/sync", get(handle_sync))
        .route("/api/accounts/profile", get(handle_profile))
        .route("/api/accounts/keys", get(handle_get_keys).post(handle_update_keys))
        .route("/api/collections", get(handle_collections))
}

/// GET /api/sync
async fn handle_sync(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<SyncData>, (StatusCode, Json<ErrorResponse>)> {
    let account = state
        .db
        .get_account_by_id(&user_id.0)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "not_found".to_string(),
                    error_description: "Account not found".to_string(),
                }),
            )
        })?;

    let ciphers = state.db.list_ciphers(&user_id.0).unwrap_or_default();
    let folders = state.db.list_folders(&user_id.0).unwrap_or_default();

    Ok(Json(SyncData {
        profile: Profile::from(account),
        folders,
        ciphers,
        domains: Domains {
            equivalent_domains: vec![],
            global_equivalent_domains: vec![],
            object: "domains".to_string(),
        },
        object: "sync".to_string(),
    }))
}

/// GET /api/accounts/profile
async fn handle_profile(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<Profile>, (StatusCode, Json<ErrorResponse>)> {
    let account = state
        .db
        .get_account_by_id(&user_id.0)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "not_found".to_string(),
                    error_description: "Account not found".to_string(),
                }),
            )
        })?;

    Ok(Json(Profile::from(account)))
}

/// GET /api/accounts/keys
async fn handle_get_keys(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<KeyPair>, (StatusCode, Json<ErrorResponse>)> {
    let account = state
        .db
        .get_account_by_id(&user_id.0)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "not_found".to_string(),
                    error_description: "Account not found".to_string(),
                }),
            )
        })?;

    Ok(Json(account.keys))
}

/// POST /api/accounts/keys
async fn handle_update_keys(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(req): Json<KeysUpdateRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .db
        .update_keys(&user_id.0, &req.encrypted_private_key, &req.public_key)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(StatusCode::OK)
}

/// GET /api/collections
async fn handle_collections(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let collections = state.db.list_collections(&user_id.0).unwrap_or_default();

    Ok(Json(serde_json::json!({
        "data": collections,
        "object": "list",
    })))
}
