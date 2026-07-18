use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use crate::api::{SharedState, UserId};
use crate::models::*;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/folders", get(handle_list_folders).post(handle_create_folder))
        .route("/api/folders/{id}", post(handle_rename_folder).delete(handle_delete_folder))
        // Android bug: requests "/apifolders" instead of "/api/folders"
        .route("/apifolders", get(handle_list_folders))
}

/// GET /api/folders
async fn handle_list_folders(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let folders = state.db.list_folders(&user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "data": folders,
        "object": "list",
    })))
}

/// POST /api/folders
async fn handle_create_folder(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(req): Json<FolderCreateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let id = state.db.create_folder(&req.name, &user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "name": req.name,
            "object": "folder",
            "revisionDate": now,
        })),
    ))
}

/// POST /api/folders/{id}
async fn handle_rename_folder(
    user_id: UserId,
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<FolderRenameRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let updated = state.db.update_folder(&id, &req.name, &user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    if !updated {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Folder not found".to_string(),
            }),
        ));
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    Ok(Json(serde_json::json!({
        "id": id,
        "name": req.name,
        "object": "folder",
        "revisionDate": now,
    })))
}

/// DELETE /api/folders/{id}
async fn handle_delete_folder(
    user_id: UserId,
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.db.delete_folder(&id, &user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Folder not found".to_string(),
            }),
        ));
    }

    Ok(StatusCode::OK)
}
