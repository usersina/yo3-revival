#![cfg(windows)]
use std::path::{Component, Prefix};

#[test]
fn original_game_loader_receives_a_normal_absolute_path_with_spaces_and_unicode() {
    let root = std::env::temp_dir().join(format!("yo3-native-path-{}-日本語 game", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("game.exe"); std::fs::write(&file, b"path identity fixture").unwrap();
    let canonical = file.canonicalize().unwrap();
    assert!(matches!(canonical.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::VerbatimDisk(_))));
    let native = yo3_launcher::runtime::native_path(&canonical).unwrap();
    let supported = native.is_absolute() && matches!(native.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)));
    let same_file = std::fs::read(&native).unwrap() == b"path identity fixture";
    std::fs::remove_dir_all(root).unwrap();
    assert!(supported, "the game's Win32 loader rejects verbatim paths: {native:?}");
    assert!(same_file, "normalizing the native path must preserve the selected file");
}
