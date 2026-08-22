use backuper_core::config::{Config, Rule};
use backuper_core::error::BackuperError;
use std::path::Path;
use tracing::info;

pub use crate::runner::RunResult;

pub async fn run_job(
    config: &Config,
    rule: &Rule,
    data_dir: &Path,
) -> Result<RunResult, BackuperError> {
    let storage_config = config
        .storages
        .iter()
        .find(|s| s.id() == rule.storage)
        .ok_or_else(|| BackuperError::Storage(format!("未找到存储配置: {}", rule.storage)))?;

    let source = crate::sources::build_source(&rule.source)?;
    let backend = crate::storage::build_storage(storage_config)?;

    let work_dir = data_dir.join("work");
    tokio::fs::create_dir_all(&work_dir).await?;

    info!(rule_id = %rule.id, "开始执行规则");
    let result = crate::runner::run_once(rule, &*source, &*backend, &work_dir).await?;
    info!(rule_id = %rule.id, key = %result.remote_key, "规则执行完成");

    if let Err(e) = crate::retention::cleanup(&*backend, rule, Some(&result.remote_key)).await {
        tracing::warn!(rule_id = %rule.id, error = %e, "retention 清理失败");
    }

    Ok(result)
}
