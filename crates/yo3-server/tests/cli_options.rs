//! The desktop reads the real server CLI, including newly added flags, rather than maintaining a second list.
use yo3_server::cli;
use clap::Parser;
#[test]
fn script_defaults_can_be_overridden_by_the_users_explicit_options() {
    let args=cli::Args::try_parse_from(["yo3-server","--bind","0.0.0.0","--data-dir","data","--log-dir","logs",
        "--bind","127.0.0.1","--data-dir","custom data","--log-dir","custom logs"]).unwrap();
    assert_eq!(args.bind.to_string(),"127.0.0.1");assert_eq!(args.data_dir.to_str(),Some("custom data"));assert_eq!(args.log_dir.to_str(),Some("custom logs"));
}
#[test]
fn the_option_catalogue_exposes_defaults_types_and_all_duel_controls() {
    let options = cli::server_options();
    for name in ["data-dir", "bind", "auth-port", "auth-cert", "auth-key", "http-port", "advertise", "game-dir", "log-dir", "no-packet-log", "diagnostics-dir", "diagnostic-parent", "players", "duel-lp", "duel-deck", "duel-extra", "duel-time", "duel-regulation", "duel-check", "regulation"] {
        assert!(options.iter().any(|o| o.name == name), "missing --{name}");
    }
    assert!(options.iter().find(|o| o.name == "duel-check").unwrap().toggle);
    assert_eq!(options.iter().find(|o| o.name == "duel-lp").unwrap().default, "8000");
    assert!(!options.iter().any(|o| o.name == "help" || o.name == "version"));
}
#[test]
fn server_options_use_the_same_validation_as_the_cli() {
    assert!(cli::validate_server_args(["--duel-lp=1000", "--duel-time=0,0", "--duel-check"]).is_ok());
    for args in [vec!["--duel-lp=wrong"], vec!["--future-typo"], vec!["add-account", "Unexpected"]] {
        assert!(cli::validate_server_args(args).is_err());
    }
}
