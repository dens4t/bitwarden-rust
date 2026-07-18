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

    let args: Vec<String> = std::env::args().collect();
    let db_path = if args.len() > 1 {
        args[1].clone()
    } else {
        "bitwarden.db".to_string()
    };
    let bind_addr = if args.len() > 2 {
        args[2].clone()
    } else {
        "0.0.0.0:8080".to_string()
    };
    let jwt_secret = if args.len() > 3 {
        args[3].clone()
    } else {
        let secret = crypto::generate_random_bytes(32);
        log::info!("No JWT secret provided, using auto-generated secret");
        secret
    };

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
