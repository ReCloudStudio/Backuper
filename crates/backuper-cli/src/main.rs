use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "backuperctl")]
struct Cli {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    println!("{:?}", cli.command);
    Ok(())
}
