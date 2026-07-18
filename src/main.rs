mod api;
mod crypto;
mod db;
mod models;

use api::{auth_middleware, AppState};
use axum::{middleware, routing::get, Router};
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // === Configuration ===
    // Priority: CLI args > Environment vars > Defaults
    let args: Vec<String> = std::env::args().collect();

    // Database path: arg[1] atau env DB_PATH atau default
    let db_path = parse_arg(&args, 1)
        .or_else(|| std::env::var("DB_PATH").ok())
        .unwrap_or_else(|| "bitwarden.db".to_string());

    // Bind address: arg[2] atau env HOST + PORT, atau env BIND_ADDR, atau default
    let bind_addr = if let Some(addr) = parse_arg(&args, 2) {
        addr
    } else if let Ok(addr) = std::env::var("BIND_ADDR") {
        addr
    } else {
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
        format!("{}:{}", host, port)
    };

    // JWT secret: arg[3] atau env JWT_SECRET atau auto-generate
    let jwt_secret = if let Some(secret) = parse_arg(&args, 3) {
        secret
    } else if let Ok(secret) = std::env::var("JWT_SECRET") {
        secret
    } else {
        let secret = crypto::generate_random_bytes(32);
        log::info!("No JWT secret provided, using auto-generated secret");
        secret
    };

    fn parse_arg(args: &[String], pos: usize) -> Option<String> {
        args.get(pos).map(|s| s.to_string())
    }

    let database = db::Database::open(Path::new(&db_path)).expect("Failed to open database");
    log::info!("Database opened at: {}", db_path);

    let state = Arc::new(AppState {
        db: database,
        jwt_secret: jwt_secret.clone(),
    });

    let app = Router::new()
        // Health check (public)
        .route("/", get(health_check))
        // Public API routes (no auth required)
        .merge(api::auth::routes())
        // Protected API routes with auth middleware
        .merge(
            Router::new()
                .merge(api::ciphers::routes())
                .merge(api::folders::routes())
                .merge(api::sync::routes())
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth_middleware,
                )),
        )
        // Layers
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    log::info!("Starting bitwarden-rs server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind address");
    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

/// GET / - Health check
async fn health_check() -> &'static str {
    "OK - bitwarden-rs running"
}
