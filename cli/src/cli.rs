use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cinematex",
    version,
    about = "Outil d'administration Cinematex (Freebox OS, TMDB et Firestore)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Appairage Freebox OS et connexion Admin Firebase
    Auth,
    /// Gestion des téléchargements torrents sur la Freebox
    Torrent {
        #[command(subcommand)]
        action: TorrentAction,
    },
    /// Scanner un dossier de la Freebox et indexer les vidéos dans Firestore
    Scan {
        /// Chemin du dossier sur la Freebox (ex: /Disque dur/Films)
        path: String,
    },
    /// Gestion des profils utilisateurs
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Gestion des droits d'accès aux programmes par profil
    Access {
        #[command(subcommand)]
        action: AccessAction,
    },
    /// Gestion des demandes d'ajout de programmes
    Request {
        #[command(subcommand)]
        action: RequestAction,
    },
    /// Ouvrir l'interface interactive dans le terminal (TUI)
    Ui,
}
#[derive(Debug, Subcommand)]
pub enum TorrentAction {
    /// Ajouter un lien magnet ou un fichier .torrent
    Add {
        /// Lien magnet ou chemin local vers un fichier .torrent
        target: String,
    },
    /// Lister les téléchargements en cours et terminés
    List,
    /// Supprimer un téléchargement par son ID Freebox
    Rm { id: usize },
}
#[derive(Debug, Subcommand)]
pub enum ProfileAction {
    /// Créer un nouveau profil
    Add { name: String },
    /// Lister les profils existants
    List,
    /// Supprimer un profil par son ID Firestore
    Rm { id: String },
}
#[derive(Debug, Subcommand)]
pub enum AccessAction {
    /// Autoriser un profil à accéder à un programme
    Grant {
        media_id: String,
        profile_id: String,
    },
    /// Retirer l'accès d'un profil à un programme
    Revoke {
        media_id: String,
        profile_id: String,
    },
}
#[derive(Debug, Subcommand)]
pub enum RequestAction {
    /// Lister les demandes de programmes en attente
    List,
    /// Rejeter une demande par son ID Firestore
    Reject { id: String },
}
