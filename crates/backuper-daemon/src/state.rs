use backuper_core::config::Config;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

pub struct InnerState {
    pub config: Arc<RwLock<Config>>,
    pub data_dir: PathBuf,
    pub pool: SqlitePool,
}

pub struct AppState {
    pub inner: Arc<InnerState>,
    pub config_path: PathBuf,
    pub scheduler: Arc<RwLock<JobScheduler>>,
    pub job_ids: Arc<RwLock<Vec<Uuid>>>,
}
