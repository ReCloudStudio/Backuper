use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::source::Source;
use std::path::Path;
use tracing::{error, info};

pub struct PostgresSource {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl PostgresSource {
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
impl Source for PostgresSource {
    async fn backup(&self, target: &Path) -> Result<(), BackuperError> {
        let temp_sql = target.with_extension("sql");
        info!(host = %self.host, database = %self.database, "开始 pg_dump");

        let mut cmd = tokio::process::Command::new("pg_dump");
        cmd.arg("-h")
            .arg(&self.host)
            .arg("-p")
            .arg(self.port.to_string())
            .arg("-d")
            .arg(&self.database)
            .arg("-f")
            .arg(&temp_sql);

        if let Some(user) = &self.username {
            cmd.arg("-U").arg(user);
        }
        if let Some(pass) = &self.password {
            cmd.env("PGPASSWORD", pass);
        }

        let status = cmd.status().await?;
        if !status.success() {
            error!(status = ?status, "pg_dump 失败");
            return Err(BackuperError::Source(format!(
                "pg_dump 退出码: {:?}",
                status
            )));
        }

        crate::compress::compress_file(&temp_sql, target).await?;
        tokio::fs::remove_file(&temp_sql).await?;
        Ok(())
    }
}
