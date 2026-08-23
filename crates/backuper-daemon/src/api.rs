use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::error;

use crate::db::list_recent_jobs;
use crate::job::execute as execute_job;
use crate::scheduler::reload as reload_config;
use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/api/status", get(status))
        .route("/api/rules", get(list_rules))
        .route("/api/run/{rule_id}", post(run_rule))
        .route("/api/reload", post(reload))
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/health", get(health))
        .route("/api/login", post(login))
        .merge(protected)
        .with_state(state)
}

async fn require_auth(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(token) = &state.api_token {
        let provided = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        let valid =
            matches!(provided, Some(v) if v.strip_prefix("Bearer ").is_some_and(|t| t == token));
        if !valid {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    Ok(next.run(request).await)
}

#[derive(Deserialize)]
struct LoginRequest {
    token: String,
}

#[derive(Serialize)]
struct LoginResponse {
    ok: bool,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let ok = match &state.api_token {
        Some(token) => &payload.token == token,
        None => true,
    };
    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };
    (status, Json(LoginResponse { ok }))
}

async fn health() -> &'static str {
    "ok"
}

async fn status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let config = state.inner.config.read().await;
    let jobs = list_recent_jobs(&state.inner.pool, 20)
        .await
        .unwrap_or_default();

    let status = json!({
        "config_path": state.config_path,
        "data_dir": state.inner.data_dir,
        "listen": config.global.listen,
        "rules_count": config.rules.len(),
        "storages_count": config.storages.len(),
        "notifiers_count": config.notifiers.len(),
        "recent_jobs": jobs,
    });

    Json(status)
}

async fn list_rules(State(state): State<Arc<AppState>>) -> Json<Value> {
    let config = state.inner.config.read().await;
    let rules: Vec<Value> = config
        .rules
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "schedule": r.schedule,
                "storage": r.storage,
                "retention": r.retention,
            })
        })
        .collect();
    Json(json!({ "rules": rules }))
}

async fn run_rule(
    State(state): State<Arc<AppState>>,
    Path(rule_id): Path<String>,
) -> impl IntoResponse {
    let config = state.inner.config.read().await;
    let rule = config.rules.iter().find(|r| r.id == rule_id).cloned();

    match rule {
        Some(rule) => {
            let inner = state.inner.clone();
            tokio::spawn(async move {
                if let Err(e) = execute_job(inner, &rule).await {
                    error!(rule_id = %rule.id, error = %e, "手动触发任务失败");
                }
            });
            (
                StatusCode::ACCEPTED,
                Json(json!({ "message": "任务已提交" })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "规则不存在" })),
        ),
    }
}

async fn reload(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match reload_config(state).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "message": "配置已重载" }))),
        Err(e) => {
            error!(error = %e, "重载配置失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        }
    }
}
