use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::error;

use crate::db::list_recent_jobs;
use crate::job::execute as execute_job;
use crate::scheduler::reload as reload_config;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/rules", get(list_rules))
        .route("/run/{rule_id}", post(run_rule))
        .route("/reload", post(reload))
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
