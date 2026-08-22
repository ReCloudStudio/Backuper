use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::source::Source;
use std::path::Path;
use tracing::{error, info};

pub struct MysqlSource {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl MysqlSource {
    pub fn new(
        host: String,
        port: u16,
        database: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            database,
            username,
            password,
        }
    }
}

#[async_trait]
impl Source for MysqlSource {
    async fn backup(&self, target: &Path) -> Result<(), BackuperError> {
        let temp_sql = target.with_extension("sql");
        info!(host = %self.host, database = %self.database, "开始 mysqldump");

        let mut cmd = tokio::process::Command::new("mysqldump");
        cmd.arg(format!("--host={}", self.host))
            .arg(format!("--port={}", self.port));

        if let Some(user) = &self.username {
            cmd.arg(format!("--user={}", user));
        }
        if let Some(pass) = &self.password {
            cmd.arg(format!("--password={}", pass));
        }

        cmd.arg("--result-file").arg(&temp_sql).arg(&self.database);

        let status = cmd.status().await?;
        if !status.success() {
            error!(status = ?status, "mysqldump 失败");
            return Err(BackuperError::Source(format!(
                "mysqldump 退出码: {:?}",
                status
            )));
        }

        crate::compress::compress_file(&temp_sql, target).await?;
        tokio::fs::remove_file(&temp_sql).await?;
        Ok(())
    }
}
