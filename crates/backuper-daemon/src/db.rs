use backuper_core::error::BackuperError;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct JobRecord {
    pub id: i64,
    pub rule_id: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub archive_key: Option<String>,
    pub error_message: Option<String>,
}

pub async fn init_pool(data_dir: &Path) -> Result<SqlitePool, BackuperError> {
    let db_path = data_dir.join("state.db");
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&url)
        .await
        .map_err(|e| BackuperError::Storage(format!("数据库连接失败: {e}")))?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| BackuperError::Storage(format!("数据库迁移失败: {e}")))?;
    Ok(pool)
}

pub async fn insert_job(pool: &SqlitePool, rule_id: &str) -> Result<i64, BackuperError> {
    let started_at = Utc::now().to_rfc3339();
    let row = sqlx::query("INSERT INTO jobs (rule_id, status, started_at) VALUES (?1, ?2, ?3)")
        .bind(rule_id)
        .bind("running")
        .bind(started_at)
        .execute(pool)
        .await
        .map_err(|e| BackuperError::Storage(format!("记录任务失败: {e}")))?;
    Ok(row.last_insert_rowid())
}

pub async fn finish_job(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    archive_key: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), BackuperError> {
    let finished_at = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE jobs SET status = ?1, finished_at = ?2, archive_key = ?3, error_message = ?4 WHERE id = ?5",
    )
    .bind(status)
    .bind(finished_at)
    .bind(archive_key)
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| BackuperError::Storage(format!("更新任务状态失败: {e}")))?;
    Ok(())
}

pub async fn list_recent_jobs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<JobRecord>, BackuperError> {
    let rows = sqlx::query_as::<_, JobRecord>(
        "SELECT id, rule_id, status, started_at, finished_at, archive_key, error_message
         FROM jobs ORDER BY started_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| BackuperError::Storage(format!("查询任务列表失败: {e}")))?;
    Ok(rows)
}
