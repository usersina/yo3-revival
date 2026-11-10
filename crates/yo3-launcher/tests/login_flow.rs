//! Compile the same connection step and byte guard included by the required client DLL.
#[test]
#[cfg(unix)]
fn skip_stock_login_preserves_native_connection_and_saved_fields() {
    let root = std::env::temp_dir().join(format!("yo3-login-flow-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/mod/login-test.c");
    let binary = root.join("login-test");
    let output = std::process::Command::new("cc")
        .args(["-O2", "-Wall", "-Wextra", "-Werror"]).arg(source).arg("-o").arg(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let output = std::process::Command::new(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(root).unwrap();
}
