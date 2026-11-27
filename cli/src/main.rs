#![forbid(unsafe_code)]

#[expect(dead_code)]
mod cli;
mod config;
mod errors;
mod firebase;
mod freebox;
#[expect(dead_code)]
mod models;
#[expect(dead_code)]
mod tmdb;
mod utils;
use crate::cli::Commands;
use crate::config::{FileSecretStore, SecretStore, Settings};
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
            let settings = Settings::load()?;
            let store = FileSecretStore::default_store()?;
            let mut secrets = store.load()?;
            if secrets.app_token.is_none() {
                let client = FreeboxClient::connect(&settings.freebox, &settings.http).await?;
                let token = client.authorize("Cinematex CLI").await?;
                secrets.app_token = Some(token);
                store.save(&secrets)?;
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
            let mut fb =
                FirebaseClient::new(project_id, api_key, &settings.firebase, &settings.http)?;
            let refresh_token = fb.sign_in(email, &password).await?;
            secrets.firebase_refresh_token = Some(refresh_token);
            store.save(&secrets)?;
            println!("Connecté à Firebase avec succès !");
        }
        Some(Commands::Torrent { .. }) => {
            todo!()
        }
        Some(Commands::Scan { path }) => {
            let secrets = FileSecretStore::default_store()?.load()?;
            let settings = Settings::load()?;
            let Some(app_token) = secrets.app_token.as_deref() else {
                return Err(AppError::Config("token d'application invalide".to_string()));
            };
            let Some(target) = path.or(settings.freebox.media_root.clone()) else {
                return Err(AppError::Config("chemin cible invalide".to_string()));
            };
            let mut client = FreeboxClient::connect(&settings.freebox, &settings.http).await?;
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
