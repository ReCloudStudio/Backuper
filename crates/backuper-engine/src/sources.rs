use backuper_core::config::SourceConfig;
use backuper_core::error::BackuperError;
use backuper_core::source::Source;

pub fn build_source(config: &SourceConfig) -> Result<Box<dyn Source>, BackuperError> {
    match config {
        SourceConfig::Directory { path } => Ok(Box::new(crate::directory::DirectorySource::new(
            path.clone(),
        ))),
        SourceConfig::Postgres {
            host,
            port,
            database,
            username,
            password,
        } => Ok(Box::new(crate::postgres::PostgresSource::new(
            host.clone(),
            *port,
            database.clone(),
            username.clone(),
            password.clone(),
        ))),
        SourceConfig::Mysql {
            host,
            port,
            database,
            username,
            password,
        } => Ok(Box::new(crate::mysql::MysqlSource::new(
            host.clone(),
            *port,
            database.clone(),
            username.clone(),
            password.clone(),
        ))),
    }
}
