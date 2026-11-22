use super::*;
use clap::Parser;

#[test]
fn parses_no_subcommand_as_none() {
    let cli = Cli::try_parse_from(["cinematex"]).unwrap();
    assert!(cli.command.is_none());
}

#[test]
fn parses_ui_and_auth_commands() {
    let ui = Cli::try_parse_from(["cinematex", "ui"]).unwrap();
    assert!(matches!(ui.command, Some(Commands::Ui)));

    let auth = Cli::try_parse_from(["cinematex", "auth"]).unwrap();
    assert!(matches!(auth.command, Some(Commands::Auth)));
}

#[test]
fn parses_scan_with_and_without_path() {
    let without = Cli::try_parse_from(["cinematex", "scan"]).unwrap();
    assert!(matches!(
        without.command,
        Some(Commands::Scan { path: None })
    ));

    let with = Cli::try_parse_from(["cinematex", "scan", "/Freebox/Films"]).unwrap();
    assert!(matches!(
        with.command,
        Some(Commands::Scan { path: Some(p) }) if p == "/Freebox/Films"
    ));
}

#[test]
fn parses_torrent_subcommands() {
    let add =
        Cli::try_parse_from(["cinematex", "torrent", "add", "magnet:?xt=urn:btih:123"]).unwrap();
    assert!(matches!(
        add.command,
        Some(Commands::Torrent {
            action: TorrentAction::Add { target }
        }) if target == "magnet:?xt=urn:btih:123"
    ));

    let list = Cli::try_parse_from(["cinematex", "torrent", "list"]).unwrap();
    assert!(matches!(
        list.command,
        Some(Commands::Torrent {
            action: TorrentAction::List
        })
    ));

    let rm = Cli::try_parse_from(["cinematex", "torrent", "rm", "42"]).unwrap();
    assert!(matches!(
        rm.command,
        Some(Commands::Torrent {
            action: TorrentAction::Rm { id: 42 }
        })
    ));
}

#[test]
fn parses_profile_access_and_request_subcommands() {
    let add = Cli::try_parse_from(["cinematex", "profile", "add", "Alice"]).unwrap();
    assert!(matches!(
        add.command,
        Some(Commands::Profile {
            action: ProfileAction::Add { name }
        }) if name == "Alice"
    ));

    let list = Cli::try_parse_from(["cinematex", "profile", "list"]).unwrap();
    assert!(matches!(
        list.command,
        Some(Commands::Profile {
            action: ProfileAction::List
        })
    ));

    let rm = Cli::try_parse_from(["cinematex", "profile", "rm", "p1"]).unwrap();
    assert!(matches!(
        rm.command,
        Some(Commands::Profile {
            action: ProfileAction::Rm { id }
        }) if id == "p1"
    ));

    let grant = Cli::try_parse_from(["cinematex", "access", "grant", "m1", "p1"]).unwrap();
    assert!(matches!(
        grant.command,
        Some(Commands::Access {
            action: AccessAction::Grant { media_id, profile_id }
        }) if media_id == "m1" && profile_id == "p1"
    ));

    let revoke = Cli::try_parse_from(["cinematex", "access", "revoke", "m1", "p1"]).unwrap();
    assert!(matches!(
        revoke.command,
        Some(Commands::Access {
            action: AccessAction::Revoke { media_id, profile_id }
        }) if media_id == "m1" && profile_id == "p1"
    ));

    let req_list = Cli::try_parse_from(["cinematex", "request", "list"]).unwrap();
    assert!(matches!(
        req_list.command,
        Some(Commands::Request {
            action: RequestAction::List
        })
    ));

    let req_reject = Cli::try_parse_from(["cinematex", "request", "reject", "r9"]).unwrap();
    assert!(matches!(
        req_reject.command,
        Some(Commands::Request {
            action: RequestAction::Reject { id }
        }) if id == "r9"
    ));
}

#[test]
fn rejects_unknown_subcommand() {
    assert!(Cli::try_parse_from(["cinematex", "unknown"]).is_err());
}
