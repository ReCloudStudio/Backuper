use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "backuperd", about = "Backuper 守护进程")]
pub struct Args {
    #[arg(short, long, default_value = "/etc/backuper/backuper.toml")]
    pub config: PathBuf,

    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,

    #[arg(short, long, default_value = "127.0.0.1:8080")]
    pub listen: String,

    #[arg(long, default_value = "./webui/.output/public")]
    pub webui_dir: PathBuf,
}
