use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub last_modified: DateTime<Utc>,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(
        &self,
        local_path: &std::path::Path,
        remote_key: &str,
    ) -> Result<(), crate::error::BackuperError>;

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, crate::error::BackuperError>;

    async fn delete(&self, remote_key: &str) -> Result<(), crate::error::BackuperError>;
}
