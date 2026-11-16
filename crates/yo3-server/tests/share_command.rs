//! `yo3-server share` (#23): what a terminal host hands a friend, the address, the identity and the invite link (with
//! `--invite`, an invite code too), never an account key. The link is the one Desktop's Add a server and the Scripts
//! helper's `--sign-in` read.
use std::process::Command;
use yo3_tunnel::invite::Invite;

fn server(data: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_yo3-server")).arg("--data-dir").arg(data).args(args)
        .env("YO3_GAME_DIR", data.join("no-game")).output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn line<'a>(out: &'a str, label: &str) -> &'a str {
    out.lines().find_map(|l| l.strip_prefix(label)).unwrap_or_else(|| panic!("no {label:?} in\n{out}")).trim()
}

#[test]
fn share_prints_the_address_identity_and_invite_link_a_friend_needs() {
    let dir = std::env::temp_dir().join(format!("yo3-share-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let data = dir.join("data");
    assert!(server(&data, &["init-auth", "localhost", "127.0.0.1", "192.0.2.7"]).0);
    let (ok, _, err) = server(&data, &["share"]);
    assert!(!ok && err.contains("--advertise"), "without the address friends reach, it says so: {err}");

    assert!(server(&data, &["set", "server-name", "Friday duels"]).0);
    let (ok, out, err) = server(&data, &["--auth-port", "7444", "--advertise", "192.0.2.7", "share"]);
    assert!(ok, "{out}\n{err}");
    let identity = yo3_tunnel::identity_from_pem(&std::fs::read(data.join("server.crt")).unwrap()).unwrap();
    assert_eq!(line(&out, "Address:"), "192.0.2.7:7444");
    assert_eq!(line(&out, "Identity:"), identity);
    let invite = Invite::parse(line(&out, "Invite link:")).unwrap();
    assert_eq!(invite, Invite::new("192.0.2.7:7444", &identity, "Friday duels").unwrap());
    assert!(!out.contains("Invite code:"), "a code only when asked: {out}");
    assert!(!out.contains("key"), "no account key: {out}");

    let (ok, out, err) = server(&data, &["--advertise", "192.0.2.7", "share", "--invite"]);
    assert!(ok, "{out}\n{err}");
    let code = line(&out, "Invite code:").split_whitespace().next().unwrap().to_string();
    let invites = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json")).unwrap().invites().to_vec();
    assert_eq!(invites.len(), 1);
    assert_eq!((invites[0].code.as_str(), invites[0].uses), (code.as_str(), Some(1)), "one sign-up");
    assert!(invites[0].expires.is_some(), "for a week");
    let _ = std::fs::remove_dir_all(dir);
}
