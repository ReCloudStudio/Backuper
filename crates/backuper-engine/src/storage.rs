use backuper_core::config::StorageConfig;
use backuper_core::error::BackuperError;
use backuper_core::storage::StorageBackend;

pub fn build_storage(config: &StorageConfig) -> Result<Box<dyn StorageBackend>, BackuperError> {
    match config {
        StorageConfig::Local { path } => {
            Ok(Box::new(crate::local::LocalStorage::new(path.clone())))
        }
        StorageConfig::Ssh {
            host,
            port,
            username,
            key,
            path,
        } => Ok(Box::new(crate::ssh::SshStorage::new(
            host.clone(),
            *port,
            username.clone(),
            key.clone(),
            path.clone(),
        ))),
    }
}
