fn main() {
    println!("cargo:rerun-if-env-changed=YO3_BUILD_VERSION");
    println!("cargo:rerun-if-env-changed=YO3_BUILD_COMMIT");
    println!("cargo:rerun-if-changed=build.rs");
    // A linked worktree's .git is a pointer file, not a directory. Missing watch paths force Cargo to rebuild forever.
    if let Some(head) = git_path("HEAD") {
        watch(&head);
        if let Some(reference) = git(&["symbolic-ref", "--quiet", "HEAD"])
            && let Some(path) = git_path(&reference) {
            // A packed reference has no loose file yet. Watch its existing parent for the next commit creating it.
            let parent = path.ancestors().find(|p| p.exists()).expect("Git reference has an existing ancestor");
            watch(parent);
            if let Some(packed) = git_path("packed-refs") { watch(&packed); }
        }
    }
    let commit = std::env::var("YO3_BUILD_COMMIT").ok().or_else(|| {
        std::process::Command::new("git").args(["rev-parse", "--short=12", "HEAD"]).output().ok()
            .filter(|out| out.status.success()).map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }).unwrap_or_else(|| "unknown".into());
    let version = std::env::var("YO3_BUILD_VERSION").unwrap_or_else(|_| format!("{}-dev", std::env::var("CARGO_PKG_VERSION").unwrap()));
    assert!(version.len() <= 64 && !version.chars().any(char::is_control), "invalid build version");
    println!("cargo:rustc-env=YO3_VERSION={version}");
    println!("cargo:rustc-env=YO3_COMMIT={commit}");
}

fn git(args: &[&str]) -> Option<String> {
    std::process::Command::new("git").args(args).output().ok().filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_owned())
}
fn git_path(name: &str) -> Option<std::path::PathBuf> {
    git(&["rev-parse", "--path-format=absolute", "--git-path", name]).map(Into::into)
}
fn watch(path: &std::path::Path) {
    if path.exists() { println!("cargo:rerun-if-changed={}", path.display()); }
}
