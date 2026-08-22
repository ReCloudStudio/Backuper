use backuper_core::config::Rule;
use backuper_core::error::BackuperError;
use backuper_core::storage::StorageBackend;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use tracing::{info, warn};

pub async fn cleanup(
    backend: &dyn StorageBackend,
    rule: &Rule,
    current_key: Option<&str>,
) -> Result<(), BackuperError> {
    if rule.retention.keep_last.is_none() && rule.retention.keep_days.is_none() {
        return Ok(());
    }

    let prefix = format!("{}_", rule.id);
    let objects = backend.list(&prefix).await?;
    if objects.is_empty() {
        return Ok(());
    }

    let mut candidates: Vec<_> = objects
        .into_iter()
        .map(|o| (parse_timestamp(&o.key).unwrap_or(o.last_modified), o))
        .collect();
    candidates.sort_by_key(|a| std::cmp::Reverse(a.0));

    let mut keep = std::collections::HashSet::new();
    if let Some(key) = current_key {
        keep.insert(key.to_string());
    }

    if let Some(n) = rule.retention.keep_last {
        for (ts, o) in candidates.iter().take(n) {
            keep.insert(o.key.clone());
            info!(rule_id = %rule.id, key = %o.key, "保留最近 {} 份之一", n);
            let _ = ts;
        }
    }

    if let Some(days) = rule.retention.keep_days {
        let cutoff = Utc::now() - chrono::Duration::days(days.into());
        for (ts, o) in &candidates {
            if *ts >= cutoff {
                keep.insert(o.key.clone());
                info!(rule_id = %rule.id, key = %o.key, "保留 {} 天内备份", days);
            }
            let _ = ts;
        }
    }

    let mut removed = 0usize;
    for (_, o) in &candidates {
        if keep.contains(&o.key) {
            continue;
        }
        if let Err(e) = backend.delete(&o.key).await {
            warn!(rule_id = %rule.id, key = %o.key, error = %e, "删除旧备份失败");
        } else {
            info!(rule_id = %rule.id, key = %o.key, "已删除旧备份");
            removed += 1;
        }
    }

    info!(rule_id = %rule.id, removed, "retention 清理完成");
    Ok(())
}

fn parse_timestamp(key: &str) -> Option<DateTime<Utc>> {
    let stem = key.rsplit_once('.').map(|(s, _)| s).unwrap_or(key);
    let (_, ts) = stem.rsplit_once('_')?;
    let naive = NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S").ok()?;
    Some(Utc.from_utc_datetime(&naive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use backuper_core::config::{RetentionConfig, Rule, SourceConfig};
    use backuper_core::storage::{ObjectMeta, StorageBackend};
    use chrono::Duration;
    use std::path::Path;

    struct FakeBackend {
        objects: std::sync::Mutex<Vec<ObjectMeta>>,
    }

    #[async_trait::async_trait]
    impl StorageBackend for FakeBackend {
        async fn store(&self, _local_path: &Path, _remote_key: &str) -> Result<(), BackuperError> {
            Ok(())
        }

        async fn list(&self, _prefix: &str) -> Result<Vec<ObjectMeta>, BackuperError> {
            Ok(self.objects.lock().unwrap().clone())
        }

        async fn delete(&self, remote_key: &str) -> Result<(), BackuperError> {
            let mut objects = self.objects.lock().unwrap();
            objects.retain(|o| o.key != remote_key);
            Ok(())
        }
    }

    fn sample_rule(retention: RetentionConfig) -> Rule {
        Rule {
            id: "docs".to_string(),
            schedule: "0 2 * * *".to_string(),
            source: SourceConfig::Directory {
                path: "/tmp".into(),
            },
            storage: "local".to_string(),
            retention,
        }
    }

    fn obj(key: &str, days_ago: i64) -> ObjectMeta {
        ObjectMeta {
            key: key.to_string(),
            last_modified: Utc::now() - Duration::days(days_ago),
        }
    }

    #[tokio::test]
    async fn keeps_last_n() {
        let backend = FakeBackend {
            objects: std::sync::Mutex::new(vec![
                obj("docs_20240101_000000.tar.zst", 5),
                obj("docs_20240102_000000.tar.zst", 4),
                obj("docs_20240103_000000.tar.zst", 3),
            ]),
        };
        let rule = sample_rule(RetentionConfig {
            keep_last: Some(2),
            keep_days: None,
        });
        cleanup(&backend, &rule, None).await.unwrap();
        let remaining = backend.objects.lock().unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|o| o.key.contains("20240102")));
        assert!(remaining.iter().any(|o| o.key.contains("20240103")));
    }

    #[tokio::test]
    async fn keeps_days() {
        let backend = FakeBackend {
            objects: std::sync::Mutex::new(vec![
                obj("docs_20240101_000000.tar.zst", 10),
                obj("docs_20240102_000000.tar.zst", 3),
                obj("docs_20240103_000000.tar.zst", 1),
            ]),
        };
        let rule = sample_rule(RetentionConfig {
            keep_last: None,
            keep_days: Some(5),
        });
        cleanup(&backend, &rule, None).await.unwrap();
        let remaining = backend.objects.lock().unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|o| o.key.contains("20240102")));
        assert!(remaining.iter().any(|o| o.key.contains("20240103")));
    }
}
