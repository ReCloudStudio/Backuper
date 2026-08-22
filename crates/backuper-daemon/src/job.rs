use backuper_core::config::Rule;
use backuper_core::error::BackuperError;
use backuper_engine::executor::RunResult;
use std::sync::Arc;
use tracing::{error, info};

use crate::db::{finish_job, insert_job};
use crate::state::InnerState;

pub async fn execute(inner: Arc<InnerState>, rule: &Rule) -> Result<RunResult, BackuperError> {
    let config = inner.config.read().await;
    let job_id = insert_job(&inner.pool, &rule.id).await?;
    info!(rule_id = %rule.id, job_id, "开始执行任务");

    let outcome = backuper_engine::executor::run_job(&config, rule, &inner.data_dir).await;

    match outcome {
        Ok(result) => {
            finish_job(
                &inner.pool,
                job_id,
                "success",
                Some(&result.remote_key),
                None,
            )
            .await?;
            info!(rule_id = %rule.id, job_id, key = %result.remote_key, "任务执行成功");
            Ok(result)
        }
        Err(ref e) => {
            let msg = e.to_string();
            if let Err(err) = finish_job(&inner.pool, job_id, "failed", None, Some(&msg)).await {
                error!(job_id, error = %err, "记录任务失败状态失败");
            }
            error!(rule_id = %rule.id, job_id, error = %e, "任务执行失败");
            outcome
        }
    }
}
