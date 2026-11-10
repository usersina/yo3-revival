fn main() {
    println!("cargo:rerun-if-env-changed=YO3_BUILD_VERSION");
    println!("cargo:rerun-if-env-changed=YO3_BUILD_COMMIT");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD")
        && let Some(reference) = head.trim().strip_prefix("ref: ") {
        println!("cargo:rerun-if-changed=../../.git/{reference}");
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
