//! Exercise the same portable C policy included by the connection DLL in ordinary workspace CI.
#[test]
#[cfg(unix)]
fn automatic_starter_preserves_files_selection_and_player_boundaries() {
    let root = std::env::temp_dir().join(format!("yo3-starter-policy-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/mod/starter-test.c");
    let binary = root.join("starter-test");
    let output = std::process::Command::new("cc")
        .args(["-O2", "-Wall", "-Wextra", "-Werror"]).arg(source).arg("-o").arg(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let output = std::process::Command::new(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(root).unwrap();
}
