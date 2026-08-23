use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/backuper")
}

fn default_listen() -> String {
    "127.0.0.1:8080".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(rename = "rule")]
    pub rules: Vec<Rule>,
    #[serde(default, rename = "storage")]
    pub storages: Vec<StorageConfig>,
    #[serde(default, rename = "notifier")]
    pub notifiers: Vec<NotifierConfig>,
}

impl Config {
    pub fn load(content: &str) -> Result<Self, crate::error::BackuperError> {
        let config: Self = toml::from_str(content)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default)]
    pub api_token: Option<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            listen: default_listen(),
            api_token: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub schedule: String,
    pub source: SourceConfig,
    pub storage: String,
    #[serde(default)]
    pub retention: RetentionConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceConfig {
    Directory {
        path: PathBuf,
    },
    Postgres {
        #[serde(default = "default_pg_host")]
        host: String,
        #[serde(default = "default_pg_port")]
        port: u16,
        database: String,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
    },
    Mysql {
        #[serde(default = "default_mysql_host")]
        host: String,
        #[serde(default = "default_mysql_port")]
        port: u16,
        database: String,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
    },
}

fn default_pg_host() -> String {
    "localhost".to_string()
}

fn default_pg_port() -> u16 {
    5432
}

fn default_mysql_host() -> String {
    "localhost".to_string()
}

fn default_mysql_port() -> u16 {
    3306
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    Local {
        id: String,
        path: PathBuf,
    },
    Ssh {
        id: String,
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        username: String,
        #[serde(default)]
        key: Option<PathBuf>,
        path: PathBuf,
    },
    S3 {
        id: String,
        endpoint: String,
        #[serde(default = "default_s3_region")]
        region: String,
        bucket: String,
        #[serde(default)]
        prefix: Option<String>,
        access_key: String,
        secret_key: String,
        #[serde(default)]
        path_style: bool,
    },
}

impl StorageConfig {
    pub fn id(&self) -> &str {
        match self {
            StorageConfig::Local { id, .. }
            | StorageConfig::Ssh { id, .. }
            | StorageConfig::S3 { id, .. } => id,
        }
    }
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

fn default_ssh_port() -> u16 {
    22
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NotifierConfig {
    Webhook {
        id: String,
        url: String,
    },
    Discord {
        id: String,
        token: String,
        channel_id: String,
    },
    Telegram {
        id: String,
        token: String,
        chat_id: String,
    },
}

impl NotifierConfig {
    pub fn id(&self) -> &str {
        match self {
            NotifierConfig::Webhook { id, .. }
            | NotifierConfig::Discord { id, .. }
            | NotifierConfig::Telegram { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RetentionConfig {
    #[serde(default)]
    pub keep_last: Option<usize>,
    #[serde(default)]
    pub keep_days: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let raw = r#"
[global]
data_dir = "/tmp/backuper"
listen = "127.0.0.1:3000"

[[rule]]
id = "docs"
schedule = "0 2 * * *"
storage = "local_backups"

[rule.source]
type = "directory"
path = "/home/docs"

[rule.retention]
keep_last = 7
keep_days = 30

[[storage]]
id = "local_backups"
type = "local"
path = "/backup"

[[notifier]]
id = "ops"
type = "webhook"
url = "https://example.com/hook"
"#;
        let config = Config::load(raw).unwrap();
        assert_eq!(config.global.data_dir, PathBuf::from("/tmp/backuper"));
        assert_eq!(config.global.listen, "127.0.0.1:3000");
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].id, "docs");
        assert_eq!(config.storages.len(), 1);
        assert_eq!(config.notifiers.len(), 1);
    }

    #[test]
    fn parse_database_defaults() {
        let raw = r#"
[[rule]]
id = "db"
schedule = "0 3 * * *"
storage = "local"

[rule.source]
type = "postgres"
database = "app"
"#;
        let config = Config::load(raw).unwrap();
        match &config.rules[0].source {
            SourceConfig::Postgres {
                host,
                port,
                database,
                ..
            } => {
                assert_eq!(host, "localhost");
                assert_eq!(*port, 5432);
                assert_eq!(database, "app");
            }
            _ => panic!("expected postgres source"),
        }
    }
}
