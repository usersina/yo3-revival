use std::path::PathBuf;
use yo3_desktop::direct;
fn scratch() -> PathBuf { let p = std::env::temp_dir().join(format!("yo3-install-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())); std::fs::create_dir(&p).unwrap(); p }
#[test]
fn reversible_install_preserves_executable_and_foreign_mods() {
    let p = scratch(); let source = p.join("bootstrap.dll"); let game = p.join("game"); std::fs::create_dir(&game).unwrap();
    std::fs::write(game.join("yo3.exe"), b"unchanged game").unwrap(); std::fs::write(&source, b"our loader").unwrap();
    std::fs::write(game.join("winmm.dll"), b"foreign mod").unwrap();
    assert!(direct::install_files(&game, &source).is_err()); assert_eq!(std::fs::read(game.join("winmm.dll")).unwrap(), b"foreign mod");
    std::fs::remove_file(game.join("winmm.dll")).unwrap(); direct::install_files(&game, &source).unwrap();
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; assert_eq!(std::fs::metadata(game.join("winmm.dll")).unwrap().permissions().mode() & 0o444, 0o444, "an elevated install must remain readable by the normal game user"); }
    direct::install_files(&game, &source).unwrap(); // idempotent owned install
    std::fs::remove_file(game.join("winmm.dll")).unwrap();
    direct::install_files(&game, &source).expect("repair a missing owned loader after an interrupted install");
    assert_eq!(std::fs::read(game.join("yo3.exe")).unwrap(), b"unchanged game");
    std::fs::write(game.join("winmm.dll"), b"another mod replaced ours").unwrap();
    assert!(direct::remove_files(&game).is_err()); assert!(game.join("winmm.dll").exists());
    std::fs::write(game.join("winmm.dll"), b"our loader").unwrap(); direct::remove_files(&game).unwrap();
    assert!(!game.join("winmm.dll").exists()); assert!(!game.join("yo3-revival-loader.json").exists());
    std::fs::remove_dir_all(p).unwrap();
}
#[cfg(unix)]
#[test]
fn installer_rejects_symlinks_and_unowned_receipts() {
    let p = scratch(); let source = p.join("bootstrap.dll"); std::fs::write(&source, b"loader").unwrap();
    std::fs::write(p.join("yo3.exe"), b"game").unwrap();
    std::os::unix::fs::symlink(&source, p.join("winmm.dll")).unwrap();
    assert!(direct::install_files(&p, &source).is_err()); assert!(direct::remove_files(&p).is_err());
    std::fs::remove_file(p.join("winmm.dll")).unwrap(); std::fs::write(p.join("yo3-revival-loader.json"), b"{}").unwrap();
    assert!(direct::install_files(&p, &source).is_err()); std::fs::remove_dir_all(p).unwrap();
}
