use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "backuperd", about = "Backuper 守护进程")]
pub struct Args {
    #[arg(short, long, default_value = "/etc/backuper/backuper.toml")]
    pub config: PathBuf,

    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,

    #[arg(short, long)]
    pub listen: Option<String>,

    #[arg(long)]
    pub webui_dir: Option<PathBuf>,
}
