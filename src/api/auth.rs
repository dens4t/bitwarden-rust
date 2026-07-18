use axum::{
    extract::{FromRequest, Request, State},
    http::{header::CONTENT_TYPE, StatusCode},
    routing::{any, get, post},
    Json, Router,
};

use std::marker::Send as SendTrait;

use crate::api::{create_token, SharedState, UserId};
use crate::crypto;
use crate::models::*;

pub fn routes() -> Router<SharedState> {
    Router::new()
        // /api routes – legacy
        .route("/api/accounts/register", post(handle_register))
        .route("/api/accounts/prelogin", post(handle_prelogin))
        .route("/api/accounts/register/send-verification-email", post(handle_send_verification_email))
        // /identity routes – used by modern Bitwarden clients
        .route("/identity/connect/token", post(handle_login))
        .route("/identity/accounts/register", post(handle_identity_register))
        .route("/identity/accounts/register/finish", post(handle_identity_register))
        .route("/identity/accounts/register/send-verification-email", post(handle_send_verification_email))
        // Stubs for extension compatibility
        .route("/api/config", any(handle_stub_empty))
        .route("/api/devices", get(handle_stub_devices).post(handle_stub_create_device))
        .route("/api/accounts/security-stamp", any(handle_stub_security_stamp))
        .route("/api/accounts/account", any(handle_stub_empty))
        .route("/api/accounts/account/profile", any(handle_stub_empty))
        .route("/api/accounts/account/keys", any(handle_stub_empty))
        .route("/api/accounts/account/security-stamp", any(handle_stub_empty))
        .route("/SDK/webLanguage", any(handle_stub_empty))
        .route("/SDK/{*rest}", any(handle_stub_empty))
        .route("/alive", any(handle_stub_empty))
        .route("/version.json", any(handle_stub_empty))
        .route("/app/version.json", any(handle_stub_empty))
        // 2FA
        .route("/api/two-factor/get-authenticator", post(handle_get_authenticator))
        .route("/api/two-factor/authenticator", post(handle_verify_authenticator))
        .route("/api/two-factor/disable", post(handle_disable_two_factor))
        .route("/api/two-factor", post(handle_two_factor_status))
}

// ── Prelogin ──────────────────────────────────────────────────

async fn handle_prelogin(
    State(state): State<SharedState>,
    Json(req): Json<PreloginRequest>,
) -> Json<PreloginResponse> {
    let (kdf, kdf_iterations) = match state.db.get_account_by_email(&req.email) {
        Ok(Some(acc)) => (acc.kdf, acc.kdf_iterations),
        _ => (crypto::DEFAULT_KDF, crypto::DEFAULT_KDF_ITERATIONS),
    };
    Json(PreloginResponse { kdf, kdf_iterations })
}

// ── Register ──────────────────────────────────────────────────

async fn handle_send_verification_email() -> &'static str {
    ""
}

/// POST /api/accounts/register
async fn handle_register(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    do_register(state, req).await
}

/// POST /identity/accounts/register and /identity/accounts/register/finish
async fn handle_identity_register(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    do_register(state, req).await
}

async fn do_register(
    state: SharedState,
    req: RegisterRequest,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let email = req.email.trim().to_lowercase();

    if let Ok(Some(_)) = state.db.get_account_by_email(&email) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "email_already_exists".to_string(),
                error_description: "An account with this email already exists.".to_string(),
            }),
        ));
    }

    let account = state.db.create_account_ext(
        &email,
        &req.name.clone().unwrap_or_default(),
        &req.resolved_hash(),
        &req.master_password_hint.clone().unwrap_or_default(),
        &req.resolved_key(),
        &req.keys.as_ref().map(|k| k.encrypted_private_key.clone()).unwrap_or_default(),
        &req.keys.as_ref().map(|k| k.public_key.clone()).unwrap_or_default(),
        req.resolved_kdf(),
        req.resolved_kdf_iterations(),
    ).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "registration_failed".to_string(),
                error_description: format!("Failed to create account: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "object": "register",
        "captchaBypassToken": "",
    })))
}

// ── Stub handlers for extension compatibility ────────────────

async fn handle_stub_empty() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

async fn handle_stub_devices() -> Json<serde_json::Value> {
    Json(serde_json::json!([]))
}

async fn handle_stub_create_device() -> Json<serde_json::Value> {
    let id = uuid::Uuid::new_v4().to_string();
    Json(serde_json::json!({
        "id": id,
        "name": "Chrome extension",
        "identifier": id,
        "type": 2,
        "status": "valid",
        "creationDate": "2026-01-01T00:00:00.000Z",
        "object": "device"
    }))
}

async fn handle_stub_security_stamp() -> Json<serde_json::Value> {
    Json(serde_json::json!({"securityStamp": uuid::Uuid::new_v4().to_string()}))
}

// ── Login – accepts both JSON and form-urlencoded ─────────────

struct LoginForm(pub LoginRequest);

impl<S: SendTrait + Sync> FromRequest<S> for LoginForm {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let content_type = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = axum::body::Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: "Failed to read request body".to_string(),
                    }),
                )
            })?;

        if content_type.starts_with("application/json") {
            let login: LoginRequest = serde_json::from_slice(&bytes).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "invalid_json".to_string(),
                        error_description: format!("Invalid JSON: {}", e),
                    }),
                )
            })?;
            Ok(LoginForm(login))
        } else {
            let str_body = String::from_utf8_lossy(&bytes);
            let login: LoginRequest = serde_urlencoded::from_str(&str_body).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "invalid_form".to_string(),
                        error_description: format!("Invalid form data: {}", e),
                    }),
                )
            })?;
            Ok(LoginForm(login))
        }
    }
}

/// POST /identity/connect/token
async fn handle_login(
    State(state): State<SharedState>,
    LoginForm(req): LoginForm,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let username = req.username.as_deref().unwrap_or("");
    let grant_type = req.grant_type.as_deref().unwrap_or("password");

    let account = if grant_type == "refresh_token" {
        let rt = req.refresh_token.as_deref().unwrap_or("");
        state.db.get_account_by_refresh_token(rt).map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: "Database error".to_string(),
            }))
        })?.ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: "Invalid refresh token".to_string(),
            }))
        })?
    } else {
        let password = req.password.as_deref().unwrap_or("");
        let account = state.db.get_account_by_email(username).map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "db_error".to_string(),
                error_description: "Database error".to_string(),
            }))
        })?.ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: "Username or password is incorrect".to_string(),
            }))
        })?;
        if password != account.master_password_hash {
            return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: "Username or password is incorrect".to_string(),
            })));
        }
        account
    };

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
        kdf_memory: None,
        kdf_parallelism: None,
        two_factor_token: None,
        object: "token".to_string(),
    }))
}

// ── 2FA ───────────────────────────────────────────────────────

async fn handle_get_authenticator(
    user_id: UserId,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let secret = crypto::generate_random_bytes(20);
    state.db.set_two_factor_secret(&user_id.0, &secret).ok();
    Ok(Json(serde_json::json!({
        "key": secret, "enabled": false, "object": "authenticator",
    })))
}

async fn handle_verify_authenticator(
    user_id: UserId,
    State(state): State<SharedState>,
    Json(req): Json<TwoFactorRequest>,
) -> Result<Json<TwoFactorResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.db.set_two_factor_secret(&user_id.0, &req.token).ok();
    Ok(Json(TwoFactorResponse {
        enabled: true,
        object: "twoFactor".to_string(),
        two_factor_providers: Some(serde_json::json!({"0": true})),
    }))
}

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
            Some(serde_json::json!({"0": true}))
        } else {
            None
        },
    })
}
