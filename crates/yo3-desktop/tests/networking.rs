use yo3_desktop::{LauncherDb, Settings, network::{BindAddress, resolve_bind}};

#[test]
fn binding_uses_current_interface_address_and_never_falls_back_to_every_interface() {
    let ethernet = BindAddress { interface_id: Some("ethernet-1".into()), label: "Ethernet".into(), address: "192.168.1.8".parse().unwrap() };
    let mut settings = Settings::default(); assert_eq!(settings.bind_address, "0.0.0.0");
    assert_eq!(resolve_bind(&settings, &[]).unwrap().to_string(), "0.0.0.0");
    settings.bind_address = "192.168.1.2".into(); settings.bind_interface = Some("ethernet-1".into());
    assert_eq!(resolve_bind(&settings, std::slice::from_ref(&ethernet)).unwrap(), ethernet.address, "DHCP address changes follow the selected interface");
    assert!(resolve_bind(&settings, &[]).is_err());
    settings.bind_address = "::1".into(); settings.bind_interface = None;
    assert_eq!(resolve_bind(&settings, &[]).unwrap().to_string(), "::1");
    settings.bind_address = "not-an-ip".into(); assert!(resolve_bind(&settings, &[]).is_err());
}

#[test]
fn existing_local_only_settings_and_nondefault_ports_survive_upgrade() {
    let root = std::env::temp_dir().join(format!("yo3-network-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap();
    let db = rusqlite::Connection::open(root.join("launcher.db")).unwrap();
    db.execute_batch("CREATE TABLE settings(id INTEGER PRIMARY KEY, body TEXT); CREATE TABLE servers(id INTEGER PRIMARY KEY, body TEXT); PRAGMA user_version=1;").unwrap();
    db.execute("INSERT INTO settings VALUES(1,?1)", [r#"{"allow_friends":false,"host_address":"localhost:17443","player_name":"OldPlayer"}"#]).unwrap(); drop(db);
    let db = LauncherDb::open(&root).unwrap(); let settings = db.settings().unwrap();
    assert_eq!(settings.bind_address, "127.0.0.1"); assert_eq!(settings.listen_port, 17443); assert_eq!(settings.player_name, "OldPlayer");
    drop(db); std::fs::remove_dir_all(root).unwrap();
}
