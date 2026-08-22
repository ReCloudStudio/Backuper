use backuper_core::config::Config;
use backuper_core::error::BackuperError;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use uuid::Uuid;

use crate::job::execute as execute_job;
use crate::state::InnerState;

fn normalize_cron(schedule: &str) -> String {
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    match parts.len() {
        5 => format!("0 {}", schedule),
        6 => schedule.to_string(),
        _ => schedule.to_string(),
    }
}

pub async fn build_scheduler(
    config: &Config,
    inner: Arc<InnerState>,
) -> Result<(JobScheduler, Vec<Uuid>), BackuperError> {
    let scheduler = JobScheduler::new()
        .await
        .map_err(|e| BackuperError::Storage(format!("调度器初始化失败: {e}")))?;

    let mut ids = Vec::with_capacity(config.rules.len());
    for rule in &config.rules {
        let schedule = normalize_cron(&rule.schedule);
        let rule_clone = rule.clone();
        let inner = inner.clone();
        let job = Job::new_cron_job_async(schedule, move |_uuid, _sched| {
            let inner = inner.clone();
            let rule = rule_clone.clone();
            Box::pin(async move {
                if let Err(e) = execute_job(inner, &rule).await {
                    error!(rule_id = %rule.id, error = %e, "定时任务执行失败");
                }
            })
        })
        .map_err(|e| BackuperError::Storage(format!("创建定时任务失败: {e}")))?;

        let id = scheduler
            .add(job)
            .await
            .map_err(|e| BackuperError::Storage(format!("注册定时任务失败: {e}")))?;
        ids.push(id);
        info!(rule_id = %rule.id, schedule = %rule.schedule, "已注册定时任务");
    }

    scheduler
        .start()
        .await
        .map_err(|e| BackuperError::Storage(format!("启动调度器失败: {e}")))?;

    Ok((scheduler, ids))
}

pub async fn reload(state: Arc<crate::state::AppState>) -> Result<(), BackuperError> {
    info!(path = %state.config_path.display(), "重新加载配置");
    let content = tokio::fs::read_to_string(&state.config_path).await?;
    let config = Config::load(&content)?;

    let mut old_ids = state.job_ids.write().await;
    {
        let mut scheduler = state.scheduler.write().await;
        for id in old_ids.drain(..) {
            if let Err(e) = scheduler.remove(&id).await {
                error!(%e, "移除旧定时任务失败");
            }
        }
        scheduler.shutdown().await.ok();
    }

    let (new_scheduler, new_ids) = build_scheduler(&config, state.inner.clone()).await?;
    *state.inner.config.write().await = config;
    *state.scheduler.write().await = new_scheduler;
    *old_ids = new_ids;

    info!("配置已重新加载");
    Ok(())
}
