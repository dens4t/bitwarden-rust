use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};

use crate::api::{create_token, SharedState, UserId};
use crate::crypto;
use crate::models::*;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/accounts/register", post(handle_register))
        .route("/identity/connect/token", post(handle_login))
        .route("/api/accounts/prelogin", post(handle_prelogin))
        .route("/api/two-factor/get-authenticator", post(handle_get_authenticator))
        .route("/api/two-factor/authenticator", post(handle_verify_authenticator))
        .route("/api/two-factor/disable", post(handle_disable_two_factor))
        .route("/api/two-factor", post(handle_two_factor_status))
}

/// GET/POST /api/accounts/prelogin
async fn handle_prelogin(
    State(state): State<SharedState>,
    Json(req): Json<PreloginRequest>,
) -> Json<PreloginResponse> {
    let (kdf, kdf_iterations) = match state.db.get_account_by_email(&req.email) {
        Ok(Some(acc)) => (acc.kdf, acc.kdf_iterations),
        _ => (crypto::DEFAULT_KDF, crypto::DEFAULT_KDF_ITERATIONS),
    };

    Json(PreloginResponse {
        kdf,
        kdf_iterations,
    })
}

/// POST /api/accounts/register
async fn handle_register(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Ok(Some(_)) = state.db.get_account_by_email(&req.email) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "email_already_exists".to_string(),
                error_description: "An account with this email already exists.".to_string(),
            }),
        ));
    }

    let account = state.db.create_account(&req).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "registration_failed".to_string(),
                error_description: format!("Failed to create account: {}", e),
            }),
        )
    })?;

    let token = create_token(&account.id, &account.email, &state.jwt_secret).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_error".to_string(),
                error_description: "Failed to generate token".to_string(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "id": account.id,
        "token": token,
        "Key": account.key,
        "privateKey": account.keys.encrypted_private_key,
        "Kdf": account.kdf,
        "KdfIterations": account.kdf_iterations,
    })))
}

/// POST /identity/connect/token
async fn handle_login(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let account = state
        .db
        .get_account_by_email(&req.username)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "db_error".to_string(),
                    error_description: "Database error".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_grant".to_string(),
                    error_description: "Username or password is incorrect".to_string(),
                }),
            )
        })?;

    // Bitwarden clients send the pre-computed PBKDF2 hash as password.
    if req.password != account.master_password_hash {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: "Username or password is incorrect".to_string(),
            }),
        ));
    }

    let access_token = create_token(&account.id, &account.email, &state.jwt_secret).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_error".to_string(),
                error_description: "Failed to generate token".to_string(),
            }),
        )
    })?;

    let refresh_token = crypto::generate_token();
    state.db.update_refresh_token(&account.id, &refresh_token).ok();

    Ok(Json(LoginResponse {
        access_token,
        expires_in: 3600,
        token_type: "Bearer".to_string(),
        refresh_token,
        key: Some(account.key),
        private_key: Some(account.keys.encrypted_private_key),
        kdf: Some(account.kdf),
        kdf_iterations: Some(account.kdf_iterations),
        two_factor_token: None,
        object: "token".to_string(),
    }))
}

/// POST /api/two-factor/get-authenticator
async fn handle_get_authenticator(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let secret = crate::crypto::generate_random_bytes(20);
    state.db.set_two_factor_secret(&user_id.0, &secret).ok();

    Ok(Json(serde_json::json!({
        "key": secret,
        "enabled": false,
        "object": "authenticator",
    })))
}

/// POST /api/two-factor/authenticator
async fn handle_verify_authenticator(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(req): Json<TwoFactorRequest>,
) -> Result<Json<TwoFactorResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.db.set_two_factor_secret(&user_id.0, &req.token).ok();

    Ok(Json(TwoFactorResponse {
        enabled: true,
        object: "twoFactor".to_string(),
        two_factor_providers: Some(serde_json::json!({
            "0": true
        })),
    }))
}

/// POST /api/two-factor/disable
async fn handle_disable_two_factor(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<TwoFactorResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.db.disable_two_factor(&user_id.0).ok();

    Ok(Json(TwoFactorResponse {
        enabled: false,
        object: "twoFactor".to_string(),
        two_factor_providers: None,
    }))
}

/// POST /api/two-factor
async fn handle_two_factor_status(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Json<TwoFactorResponse> {
    let secret = state.db.get_two_factor_secret(&user_id.0).unwrap_or_default();
    let enabled = !secret.is_empty();

    Json(TwoFactorResponse {
        enabled,
        object: "twoFactor".to_string(),
        two_factor_providers: if enabled {
            Some(serde_json::json!({ "0": true }))
        } else {
            None
        },
    })
}
