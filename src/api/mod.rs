pub mod admin;
pub mod auth;
pub mod ciphers;
pub mod folders;
pub mod orgs;
pub mod sends;
pub mod sync;

use std::marker::Send as SendTrait;

use axum::{
    extract::{FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::models::*;

/// Shared application state
pub struct AppState {
    pub db: Database,
    pub jwt_secret: String,
}

pub type SharedState = Arc<AppState>;

// ===================== JWT Claims =====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user id
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

/// Create a JWT access token for the given user.
pub fn create_token(
    user_id: &str,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now,
        exp: now + 3600, // 1 hour
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a JWT token and return the claims.
pub fn validate_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

// ===================== User ID Extractor =====================

/// Extractor that gets the authenticated user's ID from the request.
/// Must be used after the auth middleware has run.
#[derive(Debug, Clone)]
pub struct UserId(pub String);

impl<S: SendTrait + Sync> FromRequestParts<S> for UserId {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get user_id from extensions (set by auth middleware)
        if let Some(user_id) = parts.extensions.get::<UserId>() {
            return Ok(user_id.clone());
        }

        // If not in extensions, try to validate token from header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match auth_header {
            Some(_token) => {
                // We need the state to validate the token, but we don't have it here directly.
                // The auth middleware should have already validated the token.
                // If we reach here, it means the middleware didn't run or didn't set the extension.
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "unauthorized".to_string(),
                        error_description: "Authentication required".to_string(),
                    }),
                ))
            }
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_string(),
                    error_description: "Missing authorization header".to_string(),
                }),
            )),
        }
    }
}

// ===================== Auth Middleware =====================

/// Middleware that validates JWT and injects UserId into request extensions.
pub async fn auth_middleware(
    State(state): State<SharedState>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => match validate_token(token, &state.jwt_secret) {
            Ok(claims) => {
                // Inject user_id into extensions for downstream handlers
                req.extensions_mut().insert(UserId(claims.sub));
                next.run(req).await
            }
            Err(_) => {
                let err = ErrorResponse {
                    error: "invalid_token".to_string(),
                    error_description: "Invalid or expired token".to_string(),
                };
                (StatusCode::UNAUTHORIZED, Json(err)).into_response()
            }
        },
        None => {
            let err = ErrorResponse {
                error: "unauthorized".to_string(),
                error_description: "Missing or invalid authorization header".to_string(),
            };
            (StatusCode::UNAUTHORIZED, Json(err)).into_response()
        }
    }
}

// Re-import for convenience
use axum::extract::State;
