use yo3_desktop::{Settings, LauncherDb, SavedServer, preferences::Section};
fn root() -> std::path::PathBuf { std::env::temp_dir().join(format!("yo3-preferences-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())) }
#[test]
fn saving_one_section_cannot_apply_unrelated_drafts_or_change_the_play_identity() {
    let saved=Settings{game_dir:"saved game".into(),host_address:"saved.example".into(),local_account:"MyPlayer".into(),..Default::default()};
    let draft=Settings{game_dir:"new game".into(),host_address:"unfinished.example".into(),local_account:"SomeoneElse".into(),listen_port:0,detailed_logs:true,..saved.clone()};
    let game=Section::Game.merge(&saved,&draft);
    assert_eq!(game.game_dir,"new game"); assert_eq!(game.host_address,saved.host_address); assert_eq!(game.listen_port,7443); assert_eq!(game.local_account,"MyPlayer"); assert!(!game.detailed_logs);
    let logs=Section::Diagnostics.merge(&saved,&draft); assert!(logs.detailed_logs); assert_eq!(logs.game_dir,saved.game_dir);
    let host=Section::Hosting.merge(&saved,&draft); assert_eq!(host.host_address,draft.host_address); assert_eq!(host.game_dir,saved.game_dir); assert_eq!(host.local_account,saved.local_account);
}
#[test]
fn editing_and_removing_a_connection_preserves_other_connections_and_preferences() {
    let root=root(); let mut db=LauncherDb::open(&root).unwrap();
    let mut a=SavedServer{id:0,label:"A".into(),address:"a:7443".into(),server_name:"a".into(),certificate:"certificate".into(),identity:None,key:"a".repeat(20)};
    a.id=db.save_server(&a).unwrap(); let mut b=a.clone(); b.id=0; b.label="B".into(); b.id=db.save_server(&b).unwrap();
    let settings=Settings{selected_server:Some(a.id),game_dir:"my game".into(),local_account:"MyPlayer".into(),..Default::default()}; db.save_settings(&settings).unwrap();
    a.label="Renamed".into(); a.key="b".repeat(20); db.save_server(&a).unwrap();
    assert_eq!(db.servers().unwrap().len(),2); assert_eq!(db.servers().unwrap()[0].label,"Renamed");
    db.delete_server(b.id).unwrap(); assert_eq!(db.settings().unwrap(),settings);
    db.delete_server(a.id).unwrap(); let after=db.settings().unwrap(); assert_eq!(after.selected_server,None); assert_eq!(after.game_dir,"my game"); assert_eq!(after.local_account,"MyPlayer");
    assert!(db.save_server(&a).is_err(),"editing a removed connection must not pretend to succeed");
    drop(db); std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn local_and_listening_addresses_are_never_offered_as_addresses_to_share() {
    for address in ["localhost", "LOCALHOST:7443", "127.0.0.1", "127.1.2.3", "0.0.0.0", "::1", "[::]:7443", "255.255.255.255"] {
        let s=Settings{host_address:address.into(),..Default::default()}; assert!(yo3_desktop::network::share_address(&s).unwrap().is_none(),"{address}");
    }
    let s=Settings{host_address:"192.168.1.50".into(),listen_port:17443,..Default::default()};
    assert_eq!(yo3_desktop::network::share_address(&s).unwrap().unwrap().0,"192.168.1.50:17443");
}
#[test]
fn diagnostics_save_uses_persisted_settings_and_a_bare_executable_is_not_ready() {
    let root=root(); let backend=yo3_desktop::Backend::new(root.clone()).unwrap(); let game=root.join("game"); std::fs::create_dir(&game).unwrap(); std::fs::write(game.join("yo3.exe"),b"incomplete fixture").unwrap();
    let settings=Settings{game_dir:game.to_string_lossy().into(),host_address:"saved.example".into(),local_account:"Mine".into(),..Default::default()};
    LauncherDb::open(&root).unwrap().save_settings(&settings).unwrap();
    let draft=Settings{game_dir:"unfinished game".into(),host_address:"bad/address".into(),listen_port:0,detailed_logs:true,..settings.clone()};
    let saved=backend.save_preferences(Section::Diagnostics,&draft).unwrap(); assert_eq!(saved.game_dir,settings.game_dir); assert_eq!(saved.host_address,settings.host_address); assert_eq!(saved.listen_port,7443); assert!(saved.detailed_logs);
    assert!(matches!(backend.readiness(&saved),yo3_desktop::Readiness::GameProblem(_)));
    assert!(backend.save_preferences(Section::Hosting,&draft).is_err());
    assert_eq!(LauncherDb::open(&root).unwrap().settings().unwrap(),saved,"a rejected section must leave persisted settings intact");
    drop(backend); std::fs::remove_dir_all(root).unwrap();
}
