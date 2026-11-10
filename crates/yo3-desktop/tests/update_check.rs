use yo3_desktop::updates::{self, Decision, Platform};
fn release(tag: &str) -> Vec<u8> {
    serde_json::json!({"tag_name":tag,"draft":false,"prerelease":false,"body":"A new launcher release.","assets":[
        {"name":format!("yo3-revival-{tag}-linux-x86_64.tar.gz"),"size":1234,"digest":format!("sha256:{}","ab".repeat(32))},
        {"name":format!("yo3-revival-{tag}-windows-x86_64.zip"),"size":2345,"digest":format!("sha256:{}","cd".repeat(32))}
    ]}).to_string().into_bytes()
}
#[test]
fn semantic_versions_select_the_right_platform_without_downgrades_or_prerelease_surprises() {
    let Decision::Available(next) = updates::inspect("v0.2.9", Platform::LinuxX64, &release("v0.2.10")).unwrap() else { panic!("numeric version ordering") };
    assert!(next.asset.name.ends_with("linux-x86_64.tar.gz")); assert!(next.asset.url.starts_with("https://github.com/usersina/yo3-revival/releases/download/"));
    assert!(matches!(updates::inspect("0.3.0", Platform::WindowsX64, &release("v0.2.10")).unwrap(), Decision::Current { .. }));
    assert!(matches!(updates::inspect("0.2.10+local", Platform::WindowsX64, &release("v0.2.10")).unwrap(), Decision::Current { .. }));
    assert!(matches!(updates::inspect("0.3.0-dev", Platform::WindowsX64, &release("v0.2.10")).unwrap(), Decision::Development { .. }));
    assert!(matches!(updates::inspect("usability-review", Platform::WindowsX64, &release("v0.2.10")).unwrap(), Decision::Development { .. }));
    let mut value: serde_json::Value = serde_json::from_slice(&release("v0.4.0")).unwrap(); value["prerelease"] = true.into();
    assert!(updates::inspect("0.2.0", Platform::LinuxX64, &serde_json::to_vec(&value).unwrap()).is_err());
}
#[test]
fn untrusted_release_metadata_cannot_supply_commands_or_arbitrary_download_urls() {
    for tag in ["../evil", "v0.3.0/../../evil", "file:///tmp/update", "v0.3.0;run"] {
        assert!(updates::inspect("0.2.0", Platform::LinuxX64, &release(tag)).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_slice(&release("v0.3.0")).unwrap();
    value["html_url"]="file:///tmp/evil".into(); value["assets"][0]["browser_download_url"]="https://evil.invalid/update".into();
    let Decision::Available(next) = updates::inspect("0.2.0", Platform::LinuxX64, &serde_json::to_vec(&value).unwrap()).unwrap() else { panic!() };
    assert_eq!(next.url,"https://github.com/usersina/yo3-revival/releases/tag/v0.3.0"); assert!(!next.asset.url.contains("evil.invalid"));
    value["assets"][0]["digest"]="sha256:bad".into(); assert!(updates::inspect("0.2.0",Platform::LinuxX64,&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(updates::inspect("0.2.0", Platform::LinuxX64, &vec![b'x'; 1024*1024+1]).is_err());
}
