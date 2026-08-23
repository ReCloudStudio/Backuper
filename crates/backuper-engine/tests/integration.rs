mod fixtures;

use backuper_core::config::StorageConfig;
use backuper_core::source::Source;
use backuper_core::storage::StorageBackend;
use backuper_engine::directory::DirectorySource;
use backuper_engine::mysql::MysqlSource;
use backuper_engine::postgres::PostgresSource;
use backuper_engine::s3::S3Storage;
use backuper_engine::ssh::SshStorage;
use fixtures::{start_mysql, start_postgres, start_s3, start_ssh};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::process::Command;

async fn make_dummy_archive() -> (tempfile::TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let source = temp_dir.path().join("source");
    tokio::fs::create_dir_all(&source)
        .await
        .expect("创建源目录失败");
    tokio::fs::write(source.join("data.txt"), "hello backuper")
        .await
        .expect("写入测试文件失败");

    let archive = temp_dir.path().join("backup.tar.zst");
    let dir_source = DirectorySource::new(source);
    dir_source.backup(&archive).await.expect("生成测试归档失败");
    (temp_dir, archive)
}

#[tokio::test]
#[ignore]
async fn postgres_backup() {
    let _ = tracing_subscriber::fmt::try_init();
    let pg = start_postgres().await;

    let dbname = "backuper_test";
    let status = Command::new("docker")
        .args([
            "exec",
            &pg.id,
            "psql",
            "-U",
            "postgres",
            "-c",
            &format!("CREATE DATABASE {};", dbname),
        ])
        .status()
        .await
        .expect("执行 psql 失败");
    assert!(status.success(), "创建测试数据库失败");

    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let archive = temp_dir.path().join("backup.sql.zst");

    let source = PostgresSource::new(
        pg.host.clone(),
        pg.port,
        dbname.to_string(),
        Some(pg.username.clone()),
        Some(pg.password.clone()),
    );
    source.backup(&archive).await.expect("Postgres 备份失败");

    let meta = std::fs::metadata(&archive).expect("无法读取归档元数据");
    assert!(meta.len() > 0, "备份归档未生成");
}

#[tokio::test]
#[ignore]
async fn mysql_backup() {
    let _ = tracing_subscriber::fmt::try_init();
    let mysql = start_mysql().await;

    let dbname = "backuper_test";
    let status = Command::new("docker")
        .args([
            "exec",
            &mysql.id,
            "mysql",
            "-uroot",
            &format!("-p{}", mysql.password),
            "-e",
            &format!("CREATE DATABASE {};", dbname),
        ])
        .status()
        .await
        .expect("执行 mysql 失败");
    assert!(status.success(), "创建测试数据库失败");

    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let archive = temp_dir.path().join("backup.sql.zst");

    let source = MysqlSource::new(
        mysql.host.clone(),
        mysql.port,
        dbname.to_string(),
        Some(mysql.username.clone()),
        Some(mysql.password.clone()),
    );
    source.backup(&archive).await.expect("MySQL 备份失败");

    let meta = std::fs::metadata(&archive).expect("无法读取归档元数据");
    assert!(meta.len() > 0, "备份归档未生成");
}

#[tokio::test]
#[ignore]
async fn ssh_storage() {
    let _ = tracing_subscriber::fmt::try_init();
    let ssh = start_ssh().await;
    let (_temp_dir, archive) = make_dummy_archive().await;

    let storage = SshStorage::new(
        ssh.host.clone(),
        ssh.port,
        ssh.user.clone(),
        Some(ssh.key_path.clone()),
        PathBuf::from(format!("/home/{}", ssh.user)),
    );
    storage
        .store(&archive, "backup.tar.zst")
        .await
        .expect("SFTP 上传失败");

    let objects = storage.list("backup").await.expect("列出远程对象失败");
    assert!(
        objects.iter().any(|o| o.key == "backup.tar.zst"),
        "未找到上传的归档"
    );

    storage
        .delete("backup.tar.zst")
        .await
        .expect("删除远程对象失败");
    let objects = storage.list("backup").await.expect("列出远程对象失败");
    assert!(
        !objects.iter().any(|o| o.key == "backup.tar.zst"),
        "归档删除失败"
    );
}

#[tokio::test]
#[ignore]
async fn s3_storage() {
    let _ = tracing_subscriber::fmt::try_init();
    let s3 = start_s3().await;
    let (_temp_dir, archive) = make_dummy_archive().await;

    let config = StorageConfig::S3 {
        id: "test".to_string(),
        endpoint: s3.endpoint.clone(),
        region: s3.region.clone(),
        bucket: s3.bucket.clone(),
        prefix: None,
        access_key: s3.access_key.clone(),
        secret_key: s3.secret_key.clone(),
        path_style: s3.path_style,
    };
    let storage = S3Storage::new(&config).expect("初始化 S3Storage 失败");

    storage
        .store(&archive, "backup.tar.zst")
        .await
        .expect("S3 上传失败");

    let objects = storage.list("backup").await.expect("列出 S3 对象失败");
    assert!(
        objects.iter().any(|o| o.key == "backup.tar.zst"),
        "未找到上传的归档"
    );

    storage
        .delete("backup.tar.zst")
        .await
        .expect("删除 S3 对象失败");
    let objects = storage.list("backup").await.expect("列出 S3 对象失败");
    assert!(
        !objects.iter().any(|o| o.key == "backup.tar.zst"),
        "归档删除失败"
    );
}
