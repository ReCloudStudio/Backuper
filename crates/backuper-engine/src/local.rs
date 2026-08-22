use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::storage::StorageBackend;
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
}
