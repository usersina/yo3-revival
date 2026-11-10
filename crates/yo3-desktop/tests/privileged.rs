#[test]
fn protected_loader_arguments_preserve_drive_roots_spaces_and_unicode() {
    use yo3_desktop::privileged::loader_helper_arguments;
    assert_eq!(loader_helper_arguments(std::path::Path::new("C:\\"), false).unwrap(), "--loader-helper install \"C:\\\\\"");
    assert_eq!(loader_helper_arguments(std::path::Path::new("C:\\日本語 game"), true).unwrap(), "--loader-helper remove \"C:\\日本語 game\"");
    assert!(loader_helper_arguments(std::path::Path::new("bad\"path"), false).is_err());
}
#[test]
#[cfg(unix)]
fn removed_firewall_helper_is_rejected_without_opening_a_window() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-revival")).args(["--firewall-helper", "7443"])
        .env("DISPLAY", ":98765").env_remove("WAYLAND_DISPLAY").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown launcher argument"));
}
