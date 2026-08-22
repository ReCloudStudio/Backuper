use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::storage::{ObjectMeta, StorageBackend};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

pub struct LocalStorage {
    pub path: PathBuf,
}

impl LocalStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, local_path: &Path, remote_key: &str) -> Result<(), BackuperError> {
        tokio::fs::create_dir_all(&self.path).await?;
        let dest = self.path.join(remote_key);
        tokio::fs::copy(local_path, &dest).await?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, BackuperError> {
        let mut entries = tokio::fs::read_dir(&self.path).await?;
        let mut objects = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !entry.file_type().await?.is_file() || !name.starts_with(prefix) {
                continue;
            }
            let meta = entry.metadata().await?;
            let last_modified = meta.modified().unwrap_or(std::time::SystemTime::now());
            let last_modified = DateTime::<Utc>::from(last_modified);
            objects.push(ObjectMeta {
                key: name.into_owned(),
                last_modified,
            });
        }
        Ok(objects)
    }

    async fn delete(&self, remote_key: &str) -> Result<(), BackuperError> {
        let path = self.path.join(remote_key);
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn stores_file() {
        let src_dir = TempDir::new().unwrap();
        let src = src_dir.path().join("archive.tar.zst");
        std::fs::write(&src, b"payload").unwrap();

        let dst_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(dst_dir.path().to_path_buf());
        storage.store(&src, "backup.tar.zst").await.unwrap();

        let copied = dst_dir.path().join("backup.tar.zst");
        assert!(copied.exists());
        assert_eq!(std::fs::read_to_string(&copied).unwrap(), "payload");
    }

    #[tokio::test]
    async fn lists_and_deletes_files() {
        let dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(dir.path().to_path_buf());
        std::fs::write(dir.path().join("docs_20240101_000000.tar.zst"), b"a").unwrap();
        std::fs::write(dir.path().join("docs_20240102_000000.tar.zst"), b"b").unwrap();
        std::fs::write(dir.path().join("other.txt"), b"c").unwrap();

        let objects = storage.list("docs_").await.unwrap();
        assert_eq!(objects.len(), 2);

        storage.delete(&objects[0].key).await.unwrap();
        let objects = storage.list("docs_").await.unwrap();
        assert_eq!(objects.len(), 1);
    }
}
