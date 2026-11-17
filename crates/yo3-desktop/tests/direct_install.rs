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
#[test]
fn repair_refreshes_owned_loaders_and_missing_files_without_touching_the_game_or_foreign_mods() {
    let p=scratch(); let source=p.join("bootstrap.dll"); std::fs::write(p.join("yo3.exe"),b"unchanged game").unwrap();
    std::fs::write(&source,b"old owned loader").unwrap(); direct::install_files(&p,&source).unwrap();
    std::fs::write(&source,b"updated loader").unwrap();
    direct::install_files(&p,&source).expect("repair should refresh the older owned loader without manual disable/enable");
    assert_eq!(std::fs::read(p.join("winmm.dll")).unwrap(),b"updated loader");
    std::fs::remove_file(p.join("winmm.dll")).unwrap(); std::fs::write(&source,b"next loader").unwrap(); direct::install_files(&p,&source).unwrap();
    assert_eq!(std::fs::read(p.join("yo3.exe")).unwrap(),b"unchanged game");
    std::fs::write(p.join("winmm.dll"),b"foreign replacement").unwrap();
    assert!(direct::install_files(&p,&source).is_err()); assert_eq!(std::fs::read(p.join("winmm.dll")).unwrap(),b"foreign replacement");
    std::fs::remove_dir_all(p).unwrap();
}

/// A Desktop data folder with direct start set up for `game` (an owned loader there, the private files here).
fn set_up(root: &std::path::Path, game: &std::path::Path) -> std::sync::Arc<yo3_desktop::Backend> {
    let backend = yo3_desktop::Backend::new(root.to_path_buf()).unwrap();
    let source = root.join("bootstrap.dll"); std::fs::write(&source, b"our loader").unwrap();
    direct::install_files(game, &source).unwrap();
    for name in ["loader.ini", "direct.json", "direct-setup.json"] { std::fs::write(root.join("profiles").join(name), b"set up").unwrap(); }
    backend
}

#[test]
fn removal_finishes_here_even_when_the_registry_cannot_be_changed() {
    // Review (2026-11-15): a failed registry change stopped the removal with loader.ini left, and that file then
    // refused every game-folder change. The game folder and the private files go first; the registry error is still
    // reported, and the state reads as not set up.
    let p = scratch(); let game = p.join("game"); std::fs::create_dir(&game).unwrap(); std::fs::write(game.join("yo3.exe"), b"game").unwrap();
    let root = p.join("app"); let backend = set_up(&root, &game);
    direct::set_registry_for_tests(Some(|_, _| anyhow::bail!("reg.exe failed")));
    let settings = yo3_desktop::Settings { game_dir: game.to_string_lossy().into(), ..Default::default() };
    let error = backend.disable_direct(&settings).unwrap_err();
    direct::set_registry_for_tests(None);
    assert!(format!("{error:#}").contains("reg.exe failed"), "{error:#}");
    assert!(!game.join("winmm.dll").exists() && !game.join("yo3-revival-loader.json").exists());
    for name in ["loader.ini", "direct.json", "direct-setup.json"] { assert!(!root.join("profiles").join(name).exists(), "{name}"); }
    assert_eq!(backend.direct_state(&settings), direct::State::Disabled);
    std::fs::remove_dir_all(p).unwrap();
}

#[test]
fn a_registry_value_that_is_not_there_is_already_removed() {
    // reg.exe delete fails for a value that is absent; a removal must not stop on that.
    let mut calls = vec![];
    let absent = direct::delete_registry_value(&mut |args: &[&str]| { calls.push(args[0].to_string()); Ok(false) });
    assert!(absent.is_ok(), "{absent:?}");
    assert_eq!(calls, ["delete", "query"]);
    let present = direct::delete_registry_value(&mut |args: &[&str]| Ok(args[0] == "query"));
    assert!(present.is_err(), "a value still there after a failed delete is an error");
    assert!(direct::delete_registry_value(&mut |args: &[&str]| Ok(args[0] == "delete")).is_ok());
}

#[test]
fn changing_the_game_folder_takes_direct_start_out_of_the_old_one() {
    // Before, the change was refused ("Disable direct launch for the old game location before changing it"), with no
    // checkbox left to do that. The change now removes it from the old folder itself; the next Play sets it up in the
    // new one. Needs a real game for the new folder's check.
    let real = yo3_data::default_game_dir();
    if !real.join("yo3.exe").is_file() { eprintln!("skipped: no game at {}", real.display()); return }
    let p = scratch();
    // Validate a private copy; the real installation may already have the user's direct-start loader.
    let new = p.join("new game"); std::fs::create_dir_all(new.join("package")).unwrap();
    for name in ["yo3.exe", "yo3.idx"] { std::fs::copy(real.join(name), new.join(name)).unwrap(); }
    #[cfg(unix)] std::os::unix::fs::symlink(real.join("package/data002.ypk"), new.join("package/data002.ypk")).unwrap();
    #[cfg(not(unix))] std::fs::copy(real.join("package/data002.ypk"), new.join("package/data002.ypk")).unwrap();
    let old = p.join("old game"); std::fs::create_dir(&old).unwrap(); std::fs::write(old.join("yo3.exe"), b"game").unwrap();
    let root = p.join("app"); let backend = set_up(&root, &old);
    let db = yo3_desktop::LauncherDb::open(&root).unwrap();
    db.save_settings(&yo3_desktop::Settings { game_dir: old.to_string_lossy().into(), ..Default::default() }).unwrap();
    direct::set_registry_for_tests(Some(|_, _| Ok(())));
    let draft = yo3_desktop::Settings { game_dir: new.to_string_lossy().into(), ..Default::default() };
    let saved = backend.save_preferences(yo3_desktop::preferences::Section::Game, &draft);
    direct::set_registry_for_tests(None);
    assert_eq!(saved.expect("the folder change goes through").game_dir, draft.game_dir);
    assert!(!old.join("winmm.dll").exists() && !old.join("yo3-revival-loader.json").exists(), "the old folder is left clean");
    assert!(!root.join("profiles/loader.ini").exists());
    assert!(!new.join("yo3-revival-loader.json").exists(), "nothing put in the new folder until Play");
    std::fs::remove_dir_all(p).unwrap();
}
