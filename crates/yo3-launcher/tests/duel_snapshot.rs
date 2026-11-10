//! Execute the exact read-only memory reader compiled into the DLL, using synthetic 32-bit client memory.
#[test]
#[cfg(unix)]
fn native_snapshots_detect_rendering_drift_without_writing_game_memory() {
    let root = std::env::temp_dir().join(format!("yo3-duel-snapshot-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/mod/duel-snapshot-test.c");
    let binary = root.join("duel-snapshot-test");
    let output = std::process::Command::new("cc").args(["-O2", "-Wall", "-Wextra", "-Werror"])
        .arg(source).arg("-o").arg(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let output = std::process::Command::new(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(root).unwrap();
}
