#[expect(dead_code)]
mod cli;
mod config;
mod errors;
mod freebox;

use crate::cli::Commands;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};
use crate::freebox::{FileType, FreeboxClient};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    match args.command {
        None | Some(Commands::Ui) => {
            println!("!TODO! Lancement TUI ... ")
        }
        Some(Commands::Auth) => {
            let mut config = AppConfig::load()?;
            let freebox_env = config::freebox_env()?;
            let client = FreeboxClient::connect(freebox_env).await?;
            let token = client.authorize("Cinematex CLI").await?;
            config.app_token = Some(token);
            config.save()?;
            println!(
                "Appairage Freebox réussi !\nPensez à activer les droits « Accès aux fichiers » et «Téléchargements » dans Freebox OS."
            );
        }
        Some(Commands::Torrent { .. }) => {
            todo!()
        }
        Some(Commands::Scan { path }) => {
            let config = AppConfig::load()?;
            let freebox_env = config::freebox_env()?;
            let Some(app_token) = config.app_token.as_deref() else {
                return Err(AppError::Config("token d'application invalide".to_string()));
            };
            let Some(target) = path.or(config.media_root) else {
                return Err(AppError::Config("chemin cible invalide".to_string()));
            };
            let mut client = FreeboxClient::connect(freebox_env).await?;
            client.open_session(app_token).await?;
            let res = client.list_dir(&target).await?;
            for r in res {
                match r.file_type {
                    FileType::File => {
                        println!("[FILE] {} ({} o)", r.name, r.size);
                    }
                    FileType::Dir => {
                        println!("[DIR] {} ({})", r.name, r.decoded_path()?);
                    }
                }
            }
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
