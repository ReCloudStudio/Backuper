use async_trait::async_trait;
use backuper_core::config::StorageConfig;
use backuper_core::error::BackuperError;
use backuper_core::storage::{ObjectMeta, StorageBackend};
use chrono::{DateTime, Utc};
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::path::Path;

pub struct S3Storage {
    bucket: Bucket,
    prefix: String,
}

impl S3Storage {
    pub fn new(config: &StorageConfig) -> Result<Self, BackuperError> {
        let StorageConfig::S3 {
            endpoint,
            region,
            bucket,
            prefix,
            access_key,
            secret_key,
            path_style,
            ..
        } = config
        else {
            return Err(BackuperError::Storage("S3 配置类型错误".to_string()));
        };

        let region = Region::Custom {
            region: region.clone(),
            endpoint: endpoint.clone(),
        };
        let credentials = Credentials::new(
            Some(access_key.as_str()),
            Some(secret_key.as_str()),
            None,
            None,
            None,
        )
        .map_err(|e| BackuperError::Storage(format!("S3 凭证错误: {e}")))?;

        let mut bucket = Bucket::new(bucket.as_str(), region, credentials)
            .map_err(|e| BackuperError::Storage(format!("S3 bucket 初始化失败: {e}")))?;
        if *path_style {
            bucket = bucket.with_path_style();
        }

        Ok(Self {
            bucket: *bucket,
            prefix: prefix.clone().unwrap_or_default(),
        })
    }

    fn full_key(&self, remote_key: &str) -> String {
        if self.prefix.is_empty() {
            remote_key.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), remote_key)
        }
    }

    fn strip_prefix(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            key.strip_prefix(&format!("{}/", self.prefix.trim_end_matches('/')))
                .unwrap_or(key)
                .to_string()
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn store(&self, local_path: &Path, remote_key: &str) -> Result<(), BackuperError> {
        let data = tokio::fs::read(local_path).await?;
        let key = self.full_key(remote_key);
        self.bucket
            .put_object(&key, &data)
            .await
            .map_err(|e| BackuperError::Storage(format!("S3 上传失败: {e}")))?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, BackuperError> {
        let full_prefix = self.full_key(prefix);
        let results = self
            .bucket
            .list(full_prefix, Some("/".to_string()))
            .await
            .map_err(|e| BackuperError::Storage(format!("S3 列出对象失败: {e}")))?;

        let mut objects = Vec::new();
        for result in results {
            for obj in result.contents {
                let last_modified = DateTime::parse_from_rfc3339(&obj.last_modified)
                    .map(|dt| dt.into())
                    .unwrap_or_else(|_| Utc::now());
                objects.push(ObjectMeta {
                    key: self.strip_prefix(&obj.key),
                    last_modified,
                });
            }
        }
        Ok(objects)
    }

    async fn delete(&self, remote_key: &str) -> Result<(), BackuperError> {
        let key = self.full_key(remote_key);
        self.bucket
            .delete_object(&key)
            .await
            .map_err(|e| BackuperError::Storage(format!("S3 删除失败: {e}")))?;
        Ok(())
    }
}
