#![forbid(unsafe_code)]

#[expect(dead_code)]
mod cli;
mod config;
mod errors;
mod firebase;
#[expect(dead_code)]
mod firestore;
mod freebox;
#[expect(dead_code)]
mod models;
mod utils;

use crate::cli::Commands;
use crate::config::AppConfig;
use crate::errors::{AppError, Result};
use crate::firebase::FirebaseClient;
use crate::freebox::{FileType, FreeboxClient};
use clap::Parser;
use std::io::{self, Write};
#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    match args.command {
        None | Some(Commands::Ui) => {
            println!("!TODO! Lancement TUI ... ")
        }
        Some(Commands::Auth) => {
            let mut config = AppConfig::load()?;
            if config.app_token.is_none() {
                let freebox_env = config::freebox_env()?;
                let client = FreeboxClient::connect(freebox_env).await?;
                let token = client.authorize("Cinematex CLI").await?;
                config.app_token = Some(token);
                config.save()?;
                println!(
                    "Appairage Freebox réussi !\nPensez à activer les droits « Accès aux fichiers » et «Téléchargements » dans Freebox OS."
                );
            } else {
                println!("Freebox déjà appareillée !")
            }
            print!("Email admin Firebase :");
            io::stdout().flush()?;
            let mut email = String::new();
            io::stdin().read_line(&mut email)?;
            let email = email.trim();
            if email.is_empty() || !utils::is_valid_email(email) {
                return Err(AppError::FirebaseAuth("email invalide".to_string()));
            }
            let password = rpassword::prompt_password("Mot de passe admin Firebase")?;
            let (project_id, api_key) = config::firebase_env()?;
            let mut fb = FirebaseClient::new(project_id, api_key)?;
            let refresh_token = fb.sign_in(email, &password).await?;
            config.firebase_refresh_token = Some(refresh_token);
            config.save()?;
            println!("Connecté à Firebase avec succès !");
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
