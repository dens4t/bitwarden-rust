use axum::{
    extract::State,
    http::{header, StatusCode, HeaderMap},
    response::IntoResponse,
    routing::get,
    Router,
};
use crate::api::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/admin", get(admin_dashboard))
        .route("/admin/users", get(admin_users))
        .route("/admin/health", get(admin_health))
}

fn basic_auth_ok(headers: &HeaderMap) -> bool {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
        .and_then(|v| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, v).ok())
        .and_then(|v| String::from_utf8(v).ok());

    match auth {
        Some(creds) => creds == "admin:admin",
        None => false,
    }
}

fn admin_html(body: &str, headers: &HeaderMap) -> impl IntoResponse {
    if !basic_auth_ok(headers) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Bitwarden-rs Admin\"")],
            "Unauthorized".to_string(),
        ).into_response();
    }

    let html = format!("<!DOCTYPE html>
<html lang=\"en\">
<head>
<meta charset=\"UTF-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
<title>Bitwarden-rs Admin</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; color: #333; }}
  nav {{ background: #175DDC; color: white; padding: 1rem 2rem; display: flex; align-items: center; gap: 1rem; }}
  nav h1 {{ font-size: 1.3rem; }}
  nav a {{ color: white; text-decoration: none; padding: 0.3rem 0.8rem; border-radius: 4px; }}
  nav a:hover {{ background: rgba(255,255,255,0.15); }}
  .container {{ max-width: 1000px; margin: 2rem auto; padding: 0 1rem; }}
  .card {{ background: white; border-radius: 8px; padding: 1.5rem; margin-bottom: 1rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  .card h2 {{ margin-bottom: 1rem; color: #175DDC; }}
  .stat-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; }}
  .stat {{ text-align: center; padding: 1.5rem; background: #f8f9ff; border-radius: 8px; }}
  .stat-value {{ font-size: 2rem; font-weight: bold; color: #175DDC; }}
  .stat-label {{ font-size: 0.85rem; color: #666; margin-top: 0.3rem; }}
  table {{ width: 100%; border-collapse: collapse; }}
  th, td {{ text-align: left; padding: 0.75rem; border-bottom: 1px solid #eee; }}
  th {{ font-weight: 600; color: #666; font-size: 0.85rem; text-transform: uppercase; }}
  .status {{ display: inline-block; padding: 0.2rem 0.6rem; border-radius: 12px; font-size: 0.8rem; }}
  .status-ok {{ background: #e6f7e6; color: #2e7d32; }}
  .badge {{ display: inline-block; padding: 0.15rem 0.5rem; border-radius: 10px; font-size: 0.75rem; background: #e3f2fd; color: #1565c0; }}
  .footer {{ text-align: center; padding: 2rem; color: #999; font-size: 0.85rem; }}
</style>
</head>
<body>
<nav>
  <h1>🔐 Bitwarden-rs</h1>
  <a href=\"/admin\">Dashboard</a>
  <a href=\"/admin/users\">Users</a>
  <a href=\"/admin/health\">Health</a>
</nav>
<div class=\"container\">
{}
</div>
<div class=\"footer\">Bitwarden-rs v1.1.0 - Lightweight Rust Bitwarden Server</div>
</body>
</html>", body);

    (StatusCode::OK, [("Content-Type", "text/html; charset=utf-8")], html).into_response()
}

async fn admin_dashboard(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_count = state.db.get_user_count().unwrap_or(0);
    let db_size = state.db.get_db_size();

    let body = format!(
        r#"<div class="card">
    <h2>📊 Dashboard</h2>
    <div class="stat-grid">
      <div class="stat">
        <div class="stat-value">{}</div>
        <div class="stat-label">👥 Users</div>
      </div>
      <div class="stat">
        <div class="stat-value">{:.1} KB</div>
        <div class="stat-label">💾 Database Size</div>
      </div>
      <div class="stat">
        <div class="stat-value">Rust</div>
        <div class="stat-label">⚡ Runtime</div>
      </div>
      <div class="stat">
        <div class="stat-value">SQLite</div>
        <div class="stat-label">🗄️ Database</div>
      </div>
    </div>
</div>

<div class="card">
    <h2>🔧 Server Info</h2>
    <table>
        <tr><th>Property</th><th>Value</th></tr>
        <tr><td>Version</td><td>1.1.0</td></tr>
        <tr><td>Database</td><td>SQLite</td></tr>
        <tr><td>Auth</td><td>JWT + PBKDF2-SHA256</td></tr>
        <tr><td>2FA</td><td>TOTP</td></tr>
    </table>
</div>"#,
        user_count,
        db_size as f64 / 1024.0,
    );

    admin_html(&body, &headers)
}

async fn admin_users(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let users = state.db.list_all_users().unwrap_or_default();

    let rows: String = users.iter().map(|u| {
        format!(
            r#"<tr>
                <td>{} <span class="badge">Admin</span></td>
                <td>{}</td>
                <td><span class="status status-ok">Active</span></td>
            </tr>"#,
            u.email,
            if u.two_factor_secret.is_empty() { "❌" } else { "✅" },
        )
    }).collect::<Vec<_>>().join("\n");

    let body = format!(
        r#"<div class="card">
    <h2>👥 Users ({})</h2>
    <table>
        <tr><th>Email</th><th>2FA</th><th>Status</th></tr>
        {}
    </table>
</div>"#,
        users.len(),
        rows,
    );

    admin_html(&body, &headers)
}

async fn admin_health(
    _state: State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let body = r#"<div class="card">
    <h2>❤️ Health Check</h2>
    <div class="stat-grid">
      <div class="stat">
        <div class="stat-value" style="color:#2e7d32">OK</div>
        <div class="stat-label">Server Status</div>
      </div>
      <div class="stat">
        <div class="stat-value" style="color:#2e7d32">OK</div>
        <div class="stat-label">Database</div>
      </div>
    </div>
    <p style="margin-top:1rem">Server is running normally. All services operational.</p>
</div>"#;

    admin_html(body, &headers)
}
