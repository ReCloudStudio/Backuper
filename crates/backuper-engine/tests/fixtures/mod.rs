use std::path::PathBuf;
use std::process::Command as SyncCommand;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

#[allow(dead_code)]
pub async fn drop_container(id: &str) {
    let _ = Command::new("docker")
        .args(["rm", "-f", "-v", id])
        .status()
        .await;
}

async fn run_container(args: &[String]) -> (String, String) {
    let output = Command::new("docker")
        .arg("run")
        .arg("-d")
        .args(args)
        .output()
        .await
        .expect("启动 Docker 容器失败");
    let id = String::from_utf8(output.stdout)
        .expect("docker run 输出非法")
        .trim()
        .to_string();
    sleep(Duration::from_millis(500)).await;
    let port = container_host_port(&id, args).await;
    (id, port)
}

async fn container_host_port(id: &str, args: &[String]) -> String {
    let container_port = args
        .windows(2)
        .find(|w| w[0] == "-p")
        .and_then(|w| w[1].split(':').next_back())
        .expect("缺少 -p 端口映射");
    let proto_port = format!("{}/tcp", container_port);
    for _ in 0..30 {
        let output = Command::new("docker")
            .args(["port", id, &proto_port])
            .output()
            .await
            .expect("docker port 失败");
        let line = String::from_utf8_lossy(&output.stdout);
        let line = line.trim();
        if !line.is_empty() {
            return line.split(':').next_back().unwrap().to_string();
        }
        sleep(Duration::from_millis(200)).await;
    }
    panic!("无法获取容器暴露端口");
}

#[allow(dead_code)]
pub struct PostgresFixture {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub username: String,
    pub password: String,
}

impl Drop for PostgresFixture {
    fn drop(&mut self) {
        let _ = SyncCommand::new("docker")
            .args(["rm", "-f", "-v", &self.id])
            .status();
    }
}

pub async fn start_postgres() -> PostgresFixture {
    let password = "backuper_test".to_string();
    let args: Vec<String> = vec![
        "-p".to_string(),
        "0:5432".to_string(),
        "-e".to_string(),
        format!("POSTGRES_PASSWORD={}", password),
        "postgres:16".to_string(),
    ];
    let (id, port_str) = run_container(&args).await;
    let port: u16 = port_str.parse().expect("端口解析失败");

    for _ in 0..60 {
        let status = Command::new("docker")
            .args(["exec", &id, "pg_isready", "-U", "postgres"])
            .status()
            .await;
        if let Ok(status) = status
            && status.success()
        {
            return PostgresFixture {
                id,
                host: "127.0.0.1".to_string(),
                port,
                dbname: "postgres".to_string(),
                username: "postgres".to_string(),
                password,
            };
        }
        sleep(Duration::from_secs(1)).await;
    }
    panic!("Postgres 容器未在预期时间内就绪");
}

#[allow(dead_code)]
pub struct MysqlFixture {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl Drop for MysqlFixture {
    fn drop(&mut self) {
        let _ = SyncCommand::new("docker")
            .args(["rm", "-f", "-v", &self.id])
            .status();
    }
}

pub async fn start_mysql() -> MysqlFixture {
    let password = "backuper_test".to_string();
    let args: Vec<String> = vec![
        "-p".to_string(),
        "0:3306".to_string(),
        "-e".to_string(),
        format!("MYSQL_ROOT_PASSWORD={}", password),
        "mysql:8".to_string(),
    ];
    let (id, port_str) = run_container(&args).await;
    let port: u16 = port_str.parse().expect("端口解析失败");

    for _ in 0..60 {
        let status = Command::new("docker")
            .args([
                "exec",
                &id,
                "mysqladmin",
                "-uroot",
                &format!("-p{}", password),
                "ping",
            ])
            .status()
            .await;
        if let Ok(status) = status
            && status.success()
        {
            return MysqlFixture {
                id,
                host: "127.0.0.1".to_string(),
                port,
                database: String::new(),
                username: "root".to_string(),
                password,
            };
        }
        sleep(Duration::from_secs(1)).await;
    }
    panic!("MySQL 容器未在预期时间内就绪");
}

pub struct SshFixture {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: PathBuf,
    _temp_dir: tempfile::TempDir,
}

impl Drop for SshFixture {
    fn drop(&mut self) {
        let _ = SyncCommand::new("docker")
            .args(["rm", "-f", "-v", &self.id])
            .status();
    }
}

pub async fn start_ssh() -> SshFixture {
    let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
    let key_path = temp_dir.path().join("id_ed25519");
    let pubkey_path = temp_dir.path().join("id_ed25519.pub");

    let status = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-f",
            key_path.to_str().unwrap(),
            "-N",
            "",
            "-C",
            "backuper-test",
        ])
        .status()
        .await
        .expect("运行 ssh-keygen 失败");
    assert!(status.success(), "生成 SSH 密钥失败");

    let user = "backuper".to_string();
    let volume = format!(
        "{}:/home/{}/.ssh/authorized_keys:ro",
        pubkey_path.display(),
        user
    );
    let user_spec = format!("{}::1000", user);
    let args: Vec<String> = vec![
        "-p".to_string(),
        "0:22".to_string(),
        "-v".to_string(),
        volume,
        "atmoz/sftp".to_string(),
        user_spec,
    ];
    let (id, port_str) = run_container(&args).await;
    let port: u16 = port_str.parse().expect("端口解析失败");

    for _ in 0..60 {
        let status = Command::new("ssh")
            .args([
                "-i",
                key_path.to_str().unwrap(),
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=2",
                "-p",
                &port_str,
                &format!("{}@127.0.0.1", user),
                "echo",
                "ok",
            ])
            .status()
            .await;
        if let Ok(status) = status
            && status.success()
        {
            return SshFixture {
                id,
                host: "127.0.0.1".to_string(),
                port,
                user,
                key_path,
                _temp_dir: temp_dir,
            };
        }
        sleep(Duration::from_secs(1)).await;
    }
    panic!("SSH/SFTP 容器未在预期时间内就绪");
}

pub struct S3Fixture {
    pub id: String,
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub path_style: bool,
}

impl Drop for S3Fixture {
    fn drop(&mut self) {
        let _ = SyncCommand::new("docker")
            .args(["rm", "-f", "-v", &self.id])
            .status();
    }
}

pub async fn start_s3() -> S3Fixture {
    let access_key = "minioadmin".to_string();
    let secret_key = "minioadmin".to_string();
    let region = "us-east-1".to_string();
    let bucket = format!("backuper-test-{}", uuid::Uuid::new_v4());

    let args: Vec<String> = vec![
        "-p".to_string(),
        "0:9000".to_string(),
        "-e".to_string(),
        format!("MINIO_ROOT_USER={}", access_key),
        "-e".to_string(),
        format!("MINIO_ROOT_PASSWORD={}", secret_key),
        "minio/minio".to_string(),
        "server".to_string(),
        "/data".to_string(),
        "--console-address".to_string(),
        ":9001".to_string(),
    ];
    let (id, port_str) = run_container(&args).await;
    let port: u16 = port_str.parse().expect("端口解析失败");
    let endpoint = format!("http://127.0.0.1:{}", port);

    let client = reqwest::Client::new();
    for _ in 0..60 {
        if let Ok(resp) = client
            .get(format!("{}/minio/health/live", endpoint))
            .send()
            .await
            && resp.status().is_success()
        {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    unsafe {
        std::env::set_var("RUST_S3_SKIP_LOCATION_CONSTRAINT", "true");
    }
    let region_obj = s3::region::Region::Custom {
        region: region.clone(),
        endpoint: endpoint.clone(),
    };
    let credentials =
        s3::creds::Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
            .expect("S3 凭证初始化失败");
    let config = s3::bucket_ops::BucketConfiguration::default();
    let _ =
        s3::bucket::Bucket::create_with_path_style(&bucket, region_obj, credentials, config).await;

    S3Fixture {
        id,
        endpoint,
        bucket,
        access_key,
        secret_key,
        region,
        path_style: true,
    }
}
