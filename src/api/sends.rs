use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete as delete_route},
    Router,
};
use serde_json::json;

use crate::models::*;
use crate::api::{SharedState, UserId};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/sends", get(list_sends))
        .route("/api/sends", post(create_send))
        .route("/api/sends/{id}", get(get_send))
        .route("/api/sends/{id}", post(update_send))
        .route("/api/sends/{id}", delete_route(delete_send))
        .route("/api/sends/access/{access_id}", post(access_send))
}

async fn list_sends(
    State(state): State<SharedState>,
    user: UserId,
) -> Result<Json<Vec<Send>>, (StatusCode, Json<ErrorResponse>)> {
    let sends = state.db.list_sends(&user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;
    Ok(Json(sends))
}

async fn create_send(
    State(state): State<SharedState>,
    user: UserId,
    Json(req): Json<SendCreateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let send = Send {
        id: String::new(),
        name: req.name,
        name_encrypted: true,
        text: req.text,
        text_encrypted: true,
        file: req.file,
        max_access_count: req.max_access_count,
        access_count: 0,
        revision_date: String::new(),
        expiration_date: req.expiration_date,
        deletion_date: req.deletion_date,
        password: req.password,
        disabled: req.disabled,
        hide_email: req.hide_email,
        object: "send".to_string(),
    };

    let send_id = state.db.create_send(&send, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;

    // Fetch the created send
    let created = state.db.get_send(&send_id, &user.0)
        .map_err(|_| ())
        .ok()
        .flatten()
        .unwrap_or(send);

    Ok(Json(json!({
        "id": send_id,
        "name": created.name,
        "nameEncrypted": created.name_encrypted,
        "text": created.text,
        "textEncrypted": created.text_encrypted,
        "maxAccessCount": created.max_access_count,
        "accessCount": created.access_count,
        "revisionDate": created.revision_date,
        "expirationDate": created.expiration_date,
        "deletionDate": created.deletion_date,
        "password": created.password,
        "disabled": created.disabled,
        "hideEmail": created.hide_email,
        "object": "send"
    })))
}

async fn get_send(
    State(state): State<SharedState>,
    user: UserId,
    Path(id): Path<String>,
) -> Result<Json<Send>, (StatusCode, Json<ErrorResponse>)> {
    let send = state.db.get_send(&id, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Send not found".to_string(),
            }))
        })?;
    Ok(Json(send))
}

async fn update_send(
    State(state): State<SharedState>,
    user: UserId,
    Path(id): Path<String>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // For now, just return the existing send
    let send = state.db.get_send(&id, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Send not found".to_string(),
            }))
        })?;

    Ok(Json(json!({
        "id": send.id,
        "name": send.name,
        "object": "send"
    })))
}

async fn delete_send(
    State(state): State<SharedState>,
    user: UserId,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.db.delete_send(&id, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn access_send(
    State(state): State<SharedState>,
    Path(access_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Public access to a send by its access ID
    // For simplicity, just return 404 for now
    Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
        error: "not_found".to_string(),
        error_description: "Send not found or has expired".to_string(),
    })))
}
