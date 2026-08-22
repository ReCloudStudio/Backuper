use async_trait::async_trait;

#[async_trait]
pub trait Source: Send + Sync {
    async fn backup(&self, target_dir: &std::path::Path)
    -> Result<(), crate::error::BackuperError>;
}
