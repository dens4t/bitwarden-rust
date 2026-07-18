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
        .route("/api/ciphers", get(handle_list_ciphers).post(handle_create_cipher))
        .route("/api/ciphers/import", post(handle_import_ciphers))
        .route("/api/ciphers/{id}", get(handle_get_cipher).post(handle_update_cipher).delete(handle_delete_cipher))
}

/// GET /api/ciphers
async fn handle_list_ciphers(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let ciphers = state.db.list_ciphers(&user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "data": ciphers,
        "object": "list",
    })))
}

/// GET /api/ciphers/{id}
async fn handle_get_cipher(
    user_id: UserId,
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Cipher>, (StatusCode, Json<ErrorResponse>)> {
    let cipher = state.db.get_cipher(&id, &user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Cipher not found".to_string(),
            }),
        )
    })?;

    Ok(Json(cipher))
}

/// POST /api/ciphers
async fn handle_create_cipher(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(cipher): Json<Cipher>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let id = state.db.create_cipher(&cipher, &user_id.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: format!("Database error: {}", e),
            }),
        )
    })?;

    let created = state.db.get_cipher(&id, &user_id.0).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: "Failed to read back cipher".to_string(),
            }),
        )
    })?.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Created cipher not found".to_string(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(serde_json::json!(created))))
}

/// POST /api/ciphers/{id} (update via POST)
async fn handle_update_cipher(
    user_id: UserId,
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(cipher): Json<Cipher>,
) -> Result<Json<Cipher>, (StatusCode, Json<ErrorResponse>)> {
    let updated = state.db.update_cipher(&id, &cipher, &user_id.0).map_err(|e| {
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
                error_description: "Cipher not found".to_string(),
            }),
        ));
    }

    let updated_cipher = state.db.get_cipher(&id, &user_id.0).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: "Failed to read updated cipher".to_string(),
            }),
        )
    })?.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                error_description: "Updated cipher not found".to_string(),
            }),
        )
    })?;

    Ok(Json(updated_cipher))
}

/// DELETE /api/ciphers/{id}
async fn handle_delete_cipher(
    user_id: UserId,
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.db.delete_cipher(&id, &user_id.0).map_err(|e| {
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
                error_description: "Cipher not found".to_string(),
            }),
        ));
    }

    Ok(StatusCode::OK)
}

/// POST /api/ciphers/import
async fn handle_import_ciphers(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let ciphers: Vec<Cipher> = body
        .get("ciphers")
        .and_then(|c| serde_json::from_value(c.clone()).ok())
        .unwrap_or_default();

    let count = state
        .db
        .import_ciphers(&ciphers, &user_id.0)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: format!("Import error: {}", e),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "data": {
            "importCount": count,
        },
        "object": "import",
    })))
}
