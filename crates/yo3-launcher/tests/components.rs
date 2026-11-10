mod common;
#[test]
fn component_preflight_checks_architecture_interface_release_and_truncation() {
    let root = std::env::temp_dir().join(format!("yo3-component-preflight-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap();
    let dll = root.join("mod.dll");
    std::fs::write(&dll, common::pe_fixture(2)).unwrap();
    let info = yo3_launcher::components::inspect_mod(&dll).unwrap();
    assert_eq!(info.abi, 2); assert_eq!(info.release, yo3_tunnel::BUILD_VERSION);
    std::fs::write(&dll, common::pe_fixture(999)).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    let mut wrong = common::pe_fixture(2); wrong[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(&dll, wrong).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    let mut wrong = common::pe_fixture(2); wrong[524] = b'X';
    std::fs::write(&dll, wrong).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    for len in [0, 64, 152, 512, 550] {
        std::fs::write(&dll, &common::pe_fixture(2)[..len]).unwrap(); assert!(yo3_launcher::components::inspect_mod(&dll).is_err());
    }
    let game = root.join("yo3.exe"); let injector = root.join("injector.exe");
    for file in [&game, &injector, &dll] { std::fs::write(file, common::pe_fixture(2)).unwrap(); }
    let digest = yo3_launcher::components::hash(&common::pe_fixture(2));
    let manifest = serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":2,"components":{
        "injector":{"sha256":digest},"connection_mod":{"sha256":digest}}});
    std::fs::write(root.join("bundle.json"), manifest.to_string()).unwrap();
    yo3_launcher::components::preflight(&game, &injector, &dll).unwrap();
    let mut tampered = common::pe_fixture(2); tampered[700] = 1; std::fs::write(&dll, tampered).unwrap();
    assert!(yo3_launcher::components::preflight(&game, &injector, &dll).is_err(), "manifest must detect changes outside the version header too");
    std::fs::remove_dir_all(root).unwrap();
}
