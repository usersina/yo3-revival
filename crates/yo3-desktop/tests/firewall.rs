use yo3_desktop::privileged::{windows_firewall_plan, linux_firewall_plan, LinuxFirewall};
#[test]
fn protected_loader_arguments_preserve_drive_roots_spaces_and_unicode() {
    use yo3_desktop::privileged::loader_helper_arguments;
    assert_eq!(loader_helper_arguments(std::path::Path::new("C:\\"), false).unwrap(), "--loader-helper install \"C:\\\\\"");
    assert_eq!(loader_helper_arguments(std::path::Path::new("C:\\日本語 game"), true).unwrap(), "--loader-helper remove \"C:\\日本語 game\"");
    assert!(loader_helper_arguments(std::path::Path::new("bad\"path"), false).is_err());
}
#[test]
fn firewall_commands_use_a_single_tcp_port_and_never_evaluate_paths_as_script() {
    let path = std::path::Path::new(r"C:\Games & tools\$unexpected\yo3-server.exe");
    let plan = windows_firewall_plan(path, 7443).unwrap();
    assert_eq!(plan.program, "powershell.exe");
    let script = plan.args.last().unwrap();
    assert!(script.contains("-Profile Private") && script.contains("-Protocol TCP"));
    assert!(!script.contains("Remove-NetFirewallRule"), "existing firewall policy must not be removed");
    assert!(!script.contains("Games & tools") && !script.contains("$unexpected"));
    assert!(plan.env.iter().any(|(name, value)| name == "YO3_FW_PROGRAM" && value == &path.to_string_lossy()));
    assert!(windows_firewall_plan(path, 0).is_err());
    for backend in [LinuxFirewall::Firewalld, LinuxFirewall::Ufw] {
        let commands = linux_firewall_plan(backend, 17443).unwrap();
        assert!(commands.iter().all(|c| !c.args.iter().any(|a| a == "enable" || a == "disable" || a == "flush")));
        assert!(commands.iter().any(|c| c.args.iter().any(|a| a.contains("17443"))));
    }
}

#[test]
#[cfg(unix)]
fn helper_refuses_invalid_arguments_without_initializing_the_gui_or_changing_firewall_policy() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-revival")).args(["--firewall-helper", "0"])
        .env("DISPLAY", ":98765").env_remove("WAYLAND_DISPLAY").output().unwrap();
    assert_eq!(output.status.code(), Some(30));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TCP port"));
}
