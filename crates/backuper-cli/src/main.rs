use backuper_core::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "backuperctl")]
struct Cli {
    #[arg(short, long, default_value = "/etc/backuper/backuper.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start,
    Stop,
    Reload,
    Run { rule_id: String },
    Status,
    Configtest,
}

fn data_dir(config: &Config) -> PathBuf {
    config.global.data_dir.clone()
}

fn pid_path(config: &Config) -> PathBuf {
    data_dir(config).join("backuperd.pid")
}

fn base_url(config: &Config) -> String {
    format!("http://{}", config.global.listen)
}

fn daemon_path() -> PathBuf {
    let mut exe = std::env::current_exe().expect("当前可执行文件路径");
    exe.pop();
    exe.join("backuperd")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config_content = tokio::fs::read_to_string(&cli.config).await?;
    let config = Config::load(&config_content)?;

    if let Command::Configtest = cli.command {
        println!("配置语法正确");
        return Ok(());
    }

    match cli.command {
        Command::Start => start_daemon(&cli.config, &config).await,
        Command::Stop => stop_daemon(&config).await,
        Command::Reload => reload_daemon(&config).await,
        Command::Run { rule_id } => run_rule(&config, &rule_id).await,
        Command::Status => show_status(&config).await,
        Command::Configtest => unreachable!(),
    }
}

async fn start_daemon(
    config_path: &PathBuf,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let pid = pid_path(config);
    if pid.exists() {
        return Err("守护进程已在运行".into());
    }

    let daemon = daemon_path();
    let mut child = tokio::process::Command::new(&daemon)
        .arg("--config")
        .arg(config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let base = base_url(config);
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if reqwest::get(format!("{}/health", base)).await.is_ok() {
            println!("守护进程已启动 (pid: {:?})", child.id());
            return Ok(());
        }
    }

    let _ = child.kill().await;
    Err("守护进程启动失败".into())
}

async fn stop_daemon(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = pid_path(config);
    let pid = match tokio::fs::read_to_string(&pid_path).await {
        Ok(content) => content.trim().parse::<u32>()?,
        Err(_) => return Err("守护进程未运行".into()),
    };

    let output = tokio::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!(
            "发送 SIGTERM 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("守护进程已停止");
    Ok(())
}

async fn reload_daemon(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/reload", base_url(config));
    let resp = client.post(url).send().await?;
    if resp.status().is_success() {
        println!("配置已重载");
    } else {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("重载失败: {}", text).into());
    }
    Ok(())
}

async fn run_rule(config: &Config, rule_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/run/{}", base_url(config), rule_id);
    let resp = client.post(url).send().await?;
    if resp.status().is_success() {
        println!("任务 {} 已提交", rule_id);
    } else {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("提交任务失败: {}", text).into());
    }
    Ok(())
}

async fn show_status(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get(format!("{}/status", base_url(config))).await?;
    let text = resp.text().await?;
    println!("{}", text);
    Ok(())
}
