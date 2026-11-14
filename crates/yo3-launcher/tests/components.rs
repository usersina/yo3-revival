mod common;
#[test]
fn component_preflight_checks_architecture_interface_release_and_truncation() {
    let root = std::env::temp_dir().join(format!("yo3-component-preflight-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap();
    let dll = root.join("mod.dll");
    std::fs::write(&dll, common::pe_fixture(yo3_tunnel::MOD_ABI as u32)).unwrap();
    let info = yo3_launcher::components::inspect_mod(&dll).unwrap();
    assert_eq!(info.abi, yo3_tunnel::MOD_ABI); assert_eq!(info.release, yo3_tunnel::BUILD_VERSION);
    std::fs::write(&dll, common::pe_fixture(999)).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    let mut wrong = common::pe_fixture(yo3_tunnel::MOD_ABI as u32); wrong[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(&dll, wrong).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    let mut wrong = common::pe_fixture(yo3_tunnel::MOD_ABI as u32); wrong[524] = b'X';
    std::fs::write(&dll, wrong).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    for len in [0, 64, 152, 512, 550] {
        std::fs::write(&dll, &common::pe_fixture(yo3_tunnel::MOD_ABI as u32)[..len]).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    }
    let game = root.join("yo3.exe"); let injector = root.join("injector.exe");
    std::fs::write(&game, common::game_fixture()).unwrap();
    for file in [&injector, &dll] { std::fs::write(file, common::pe_fixture(yo3_tunnel::MOD_ABI as u32)).unwrap(); }
    let digest = yo3_launcher::components::hash(&common::pe_fixture(yo3_tunnel::MOD_ABI as u32));
    let manifest = serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,"components":{
        "injector":{"sha256":digest},"connection_mod":{"sha256":digest}}});
    std::fs::write(root.join("bundle.json"), manifest.to_string()).unwrap();
    yo3_launcher::components::preflight(&game, &injector, &dll).unwrap();
    let mut tampered = common::pe_fixture(yo3_tunnel::MOD_ABI as u32); tampered[700] = 1; std::fs::write(&dll, tampered).unwrap();
    assert!(yo3_launcher::components::preflight(&game, &injector, &dll).is_err(), "manifest must detect changes outside the version header too");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn game_validation_rejects_unsupported_hook_layouts_before_startup() {
    let root = std::env::temp_dir().join(format!("yo3-game-preflight-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let game = root.join("yo3.exe"); let injector = root.join("injector.exe"); let dll = root.join("auth.dll");
    for path in [&injector, &dll] { std::fs::write(path, common::pe_fixture(yo3_tunnel::MOD_ABI as u32)).unwrap(); }
    let digest = yo3_launcher::components::hash(&common::pe_fixture(yo3_tunnel::MOD_ABI as u32));
    std::fs::write(root.join("bundle.json"), serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
        "components":{"injector":{"sha256":digest},"connection_mod":{"sha256":digest}}}).to_string()).unwrap();
    let valid = common::game_fixture(); std::fs::write(&game, &valid).unwrap();
    yo3_launcher::components::preflight(&game, &injector, &dll).unwrap();
    for at in [0x6964b, 0xf44bd, 0x114212, 0x1159ea, 180] {
        let mut changed = valid.clone(); changed[at] ^= 1; std::fs::write(&game, changed).unwrap();
        let error = yo3_launcher::components::preflight(&game, &injector, &dll)
            .expect_err("an incompatible game must not pass validation");
        assert!(format!("{error:#}").contains("1.153"), "{error:#}");
    }
    // A mapped section cannot point outside the file or claim zero-filled bytes as an executable hook.
    for length in [512, 0x1159ec] {
        std::fs::write(&game, &valid[..length]).unwrap();
        assert!(yo3_launcher::components::preflight(&game, &injector, &dll).is_err());
    }
    // RVAs are mapped through the section table, not treated as file offsets.
    let mut shifted = valid.clone(); shifted.splice(0x1000..0x1000, [0; 512]);
    let section = 128+24+224;
    shifted[section+20..section+24].copy_from_slice(&0x1200u32.to_le_bytes());
    std::fs::write(&game, &shifted).unwrap(); yo3_launcher::components::inspect_game(&game).unwrap();
    shifted[section+36..section+40].copy_from_slice(&0x40000040u32.to_le_bytes());
    std::fs::write(&game, shifted).unwrap(); assert!(yo3_launcher::components::inspect_game(&game).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
