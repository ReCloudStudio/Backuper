use backuper_core::config::Rule;
use backuper_core::error::BackuperError;
use backuper_core::source::Source;
use backuper_core::storage::StorageBackend;
use chrono::Local;
use std::path::Path;
use tracing::info;

pub struct RunResult {
    pub archive_path: std::path::PathBuf,
    pub remote_key: String,
}

pub async fn run_once<S, B>(
    rule: &Rule,
    source: &S,
    backend: &B,
    work_dir: &Path,
) -> Result<RunResult, BackuperError>
where
    S: Source + ?Sized,
    B: StorageBackend + ?Sized,
{
    tokio::fs::create_dir_all(work_dir).await?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let remote_key = format!("{}_{}.tar.zst", rule.id, timestamp);
    let archive_path = work_dir.join(&remote_key);

    info!(rule_id = %rule.id, path = %archive_path.display(), "开始备份");
    source.backup(&archive_path).await?;
    backend.store(&archive_path, &remote_key).await?;
    info!(rule_id = %rule.id, key = %remote_key, "备份完成");

    Ok(RunResult {
        archive_path,
        remote_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::directory::DirectorySource;
    use crate::local::LocalStorage;
    use backuper_core::config::{RetentionConfig, SourceConfig};
    use tempfile::TempDir;

    #[tokio::test]
    async fn end_to_end_directory_backup() {
        let src_dir = TempDir::new().unwrap();
        std::fs::write(src_dir.path().join("data.txt"), "important").unwrap();

        let work_dir = TempDir::new().unwrap();
        let storage_dir = TempDir::new().unwrap();

        let rule = Rule {
            id: "docs".to_string(),
            schedule: "0 2 * * *".to_string(),
            source: SourceConfig::Directory {
                path: src_dir.path().to_path_buf(),
            },
            storage: "local".to_string(),
            retention: RetentionConfig::default(),
        };

        let source = DirectorySource::new(src_dir.path().to_path_buf());
        let backend = LocalStorage::new(storage_dir.path().to_path_buf());

        let result = run_once(&rule, &source, &backend, work_dir.path())
            .await
            .unwrap();

        assert!(result.archive_path.exists());
        assert!(storage_dir.path().join(&result.remote_key).exists());
        assert!(result.remote_key.starts_with("docs_"));
        assert!(result.remote_key.ends_with(".tar.zst"));
    }
}
