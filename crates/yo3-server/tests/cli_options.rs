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
    for name in ["data-dir", "bind", "auth-port", "auth-cert", "auth-key", "http-port", "advertise", "game-dir", "log-dir", "no-packet-log", "diagnostics-dir", "diagnostic-parent", "players", "duel-lp", "duel-deck", "duel-extra", "duel-time", "duel-regulation", "duel-check", "config"] {
        assert!(options.iter().any(|o| o.name == name), "missing --{name}");
    }
    assert!(options.iter().find(|o| o.name == "duel-check").unwrap().toggle);
    assert_eq!(options.iter().find(|o| o.name == "duel-lp").unwrap().default, "", "a testing aid, off unless given");
    // Live settings (#24) are not flags: `yo3-server set` changes them, on a running server too.
    for live in ["regulation", "registration", "name", "min-launcher"] {
        assert!(!options.iter().any(|o| o.name == live), "--{live} is a live setting");
    }
    assert!(!options.iter().any(|o| o.name == "help" || o.name == "version"));
}
#[test]
fn server_options_use_the_same_validation_as_the_cli() {
    assert!(cli::validate_server_args(["--duel-lp=1000", "--duel-time=0,0", "--duel-check"]).is_ok());
    for args in [vec!["--duel-lp=wrong"], vec!["--future-typo"], vec!["add-account", "Unexpected"]] {
        assert!(cli::validate_server_args(args).is_err());
    }
}

#[test]
fn dashboard_listener_is_opt_in_and_has_no_public_bind_option() {
    let defaults = cli::Args::try_parse_from(["yo3-server"]).unwrap();
    assert_eq!(defaults.dashboard_port, None);
    assert_eq!(cli::Args::try_parse_from(["yo3-server", "--dashboard-port", "0"]).unwrap().dashboard_port, Some(0));
    assert!(cli::validate_server_args(["--dashboard-port", "18081"]).is_ok());
    for args in [vec!["--dashboard-port", "65536"], vec!["--dashboard-port", "-1"],
        vec!["--dashboard-bind", "0.0.0.0"]] {
        assert!(cli::validate_server_args(args).is_err());
    }
}

#[test]
fn dashboard_socket_configuration_can_be_split_between_file_and_flags() {
    assert!(cli::validate_server_args(["--dashboard-origin","https://staff.example"]).is_ok());
    assert!(cli::validate_server_args(["--dashboard-socket","/run/yo3/staff.sock"]).is_ok());
    assert!(cli::validate_server_args(["--dashboard-socket","/run/yo3/staff.sock","--dashboard-port","18081"]).is_err());
}
