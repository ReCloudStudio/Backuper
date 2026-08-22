use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(
        &self,
        local_path: &std::path::Path,
        remote_key: &str,
    ) -> Result<(), crate::error::BackuperError>;
}
