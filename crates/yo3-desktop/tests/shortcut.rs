//! Settings → Game's shortcut row knows when the shortcut is already there (#31), so it offers to make it again rather
//! than to add one. Nextest runs each test in its own process, so this one may point HOME and XDG_DATA_HOME elsewhere.
#![cfg(not(windows))]

#[test]
fn the_shortcut_is_found_once_made() {
    let root = std::env::temp_dir().join(format!("yo3-shortcut-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("home")).unwrap();
    // SAFETY: no other thread of this test process reads the environment.
    unsafe {
        std::env::set_var("HOME", root.join("home"));
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
    }
    assert!(!yo3_desktop::shortcut::exists());
    let made = yo3_desktop::shortcut::create(&root.join("yo3-revival")).unwrap();
    assert!(made.iter().all(|p| p.starts_with(&root)), "{made:?}");
    assert!(yo3_desktop::shortcut::exists());
    std::fs::remove_dir_all(root).unwrap();
}
