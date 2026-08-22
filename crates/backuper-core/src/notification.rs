use async_trait::async_trait;

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, message: &str) -> Result<(), crate::error::BackuperError>;
}
