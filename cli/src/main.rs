mod cli;
mod config;
mod errors;
mod freebox;

use crate::cli::Commands;
use crate::errors::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    match args.command {
        None | Some(Commands::Ui) => {
            println!("!TODO! Lancement TUI ... ")
        }
        Some(Commands::Auth) => {
            todo!()
        }
        Some(Commands::Torrent { .. }) => {
            todo!()
        }
        Some(Commands::Scan { .. }) => {
            todo!()
        }
        Some(Commands::Profile { .. }) => {
            todo!()
        }
        Some(Commands::Access { .. }) => {
            todo!()
        }
        Some(Commands::Request { .. }) => {
            todo!()
        }
    }
    Ok(())
}
