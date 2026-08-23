mod api;
mod args;
mod assets;
mod db;
mod job;
mod pid;
mod scheduler;
mod state;

use clap::Parser;
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot};
use tracing::{error, info};

use crate::args::Args;
use crate::db::init_pool;
use crate::pid::{is_running, remove_pid, write_pid};
use crate::scheduler::{build_scheduler, reload as reload_config};
use crate::state::{AppState, InnerState};
use backuper_core::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let config_content = tokio::fs::read_to_string(&args.config).await?;
    let config = Config::load(&config_content)?;
    let listen = args
        .listen
        .clone()
        .unwrap_or_else(|| config.global.listen.clone());

    let data_dir = args
        .data_dir
        .unwrap_or_else(|| config.global.data_dir.clone());
    tokio::fs::create_dir_all(&data_dir).await?;

    let pid_path = data_dir.join("backuperd.pid");
    if is_running(&pid_path) {
        return Err("守护进程已在运行".into());
    }

    write_pid(&pid_path)?;
    info!(path = %args.config.display(), "backuperd 启动");

    let pool = init_pool(&data_dir).await?;
    let config = Arc::new(RwLock::new(config));
    let inner = Arc::new(InnerState {
        config: config.clone(),
        data_dir: data_dir.clone(),
        pool,
    });

    let (scheduler, job_ids) = build_scheduler(&*config.read().await, inner.clone()).await?;

    let state = Arc::new(AppState {
        inner,
        config_path: args.config.clone(),
        scheduler: Arc::new(RwLock::new(scheduler)),
        job_ids: Arc::new(RwLock::new(job_ids)),
        api_token: config.read().await.global.api_token.clone(),
    });

    let app = api::router(state.clone());
    let app = if let Some(webui_dir) = args.webui_dir.as_ref().filter(|p| p.exists()) {
        let index = webui_dir.join("index.html");
        app.fallback_service(
            tower_http::services::ServeDir::new(webui_dir)
                .fallback(tower_http::services::ServeFile::new(&index)),
        )
    } else {
        app.fallback(crate::assets::serve)
    };

    let listener = tokio::net::TcpListener::bind(&listen).await?;
    info!(addr = %listen, "HTTP API 监听中");

    let server = axum::serve(listener, app);
    let server_handle = tokio::spawn(server.into_future());
    let mut server_handle_opt = Some(server_handle);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let signal_state = state.clone();

    tokio::spawn(async move {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .expect("SIGHUP handler");
        let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("SIGINT handler");

        loop {
            tokio::select! {
                _ = term.recv() => {
                    info!("收到 SIGTERM，开始关闭");
                    let _ = shutdown_tx.send(());
                    break;
                }
                _ = int.recv() => {
                    info!("收到 SIGINT，开始关闭");
                    let _ = shutdown_tx.send(());
                    break;
                }
                _ = hup.recv() => {
                    info!("收到 SIGHUP，重新加载配置");
                    if let Err(e) = reload_config(signal_state.clone()).await {
                        error!(error = %e, "配置重载失败");
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = &mut shutdown_rx => {}
        result = server_handle_opt.take().unwrap() => {
            result??;
        }
    }

    info!("backuperd 关闭中");
    {
        let mut scheduler = state.scheduler.write().await;
        if let Err(e) = scheduler.shutdown().await {
            error!(error = %e, "关闭调度器失败");
        }
    }
    if let Some(handle) = server_handle_opt.take() {
        handle.abort();
    }
    remove_pid(&pid_path)?;
    info!("backuperd 已退出");

    Ok(())
}
