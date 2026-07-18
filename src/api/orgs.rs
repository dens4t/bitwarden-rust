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
        // Organizations
        .route("/api/organizations", get(list_organizations))
        .route("/api/organizations", post(create_organization))
        .route("/api/organizations/{id}/collections", get(list_org_collections))
        .route("/api/organizations/{id}/collections", post(create_org_collection))
        .route("/api/collections/{id}", delete_route(delete_collection))
        // User organizations (for sync)
        .route("/api/organizations/{id}/users", get(list_org_users))
}

async fn list_organizations(
    State(state): State<SharedState>,
    user: UserId,
) -> Result<Json<Vec<Organization>>, (StatusCode, Json<ErrorResponse>)> {
    let orgs = state.db.list_organizations(&user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;
    Ok(Json(orgs))
}

async fn create_organization(
    State(state): State<SharedState>,
    user: UserId,
    Json(req): Json<OrganizationCreateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = state.db.create_organization(&req.name, &req.billing_email, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;

    Ok(Json(json!({
        "id": org_id,
        "name": req.name,
        "object": "organization"
    })))
}

async fn list_org_collections(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
    _user: UserId,
) -> Result<Json<Vec<Collection>>, (StatusCode, Json<ErrorResponse>)> {
    let collections = state.db.get_org_collections(&org_id)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;
    Ok(Json(collections))
}

async fn create_org_collection(
    State(state): State<SharedState>,
    Path(org_id): Path<String>,
    user: UserId,
    Json(req): Json<CollectionCreateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let coll_id = state.db.create_collection(&req.name, &org_id, &user.0)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;

    Ok(Json(json!({
        "id": coll_id,
        "name": req.name,
        "organizationId": org_id,
        "object": "collection"
    })))
}

async fn delete_collection(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    _user: UserId,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.db.delete_collection(&id)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "database_error".to_string(),
                error_description: e.to_string(),
            }))
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_org_users(
    State(state): State<SharedState>,
    Path(_org_id): Path<String>,
    _user: UserId,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    // Return empty list for now
    Ok(Json(vec![]))
}
