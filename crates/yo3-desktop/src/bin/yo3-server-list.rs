//! The publishing side of the official server list (#30): make a signing key, sign a list, check a signed file.
//!
//! ```text
//! yo3-server-list keygen KEY            a new ed25519 key in KEY (keep it private, off the repository);
//!                                       prints the public key for multiplayer::OFFICIAL_LIST_KEYS
//! yo3-server-list sign LIST KEY OUT     LIST is {"version": N, "servers": [{name, address, identities, ...}]}
//! yo3-server-list verify FILE PUBLIC... checks FILE against one or more public keys (hex)
//! ```
use std::path::Path;

use anyhow::{Context, bail};
use yo3_desktop::multiplayer::{ServerList, new_list_key, sign_list, verify_list};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["keygen", key] => {
            let (private, public) = new_list_key()?;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            std::io::Write::write_all(&mut options.open(key).with_context(|| format!("{key} (it must not exist yet)"))?, &private)?;
            println!("{public}");
        }
        ["sign", list, key, out] => {
            let list: ServerList = serde_json::from_slice(&std::fs::read(list)?).context("the list")?;
            let signed = sign_list(&list, &std::fs::read(key)?)?;
            std::fs::write(out, &signed)?;
            println!("signed {} server(s), version {}, into {out}", list.servers.len(), list.version);
        }
        ["verify", file, keys @ ..] if !keys.is_empty() => {
            let list = verify_list(&std::fs::read(Path::new(file))?, keys)?;
            for server in &list.servers { println!("{}\t{}\t{}", server.name, server.address, server.identities.join(",")) }
            println!("version {}: the signature is good", list.version);
        }
        _ => bail!("usage: yo3-server-list keygen KEY | sign LIST KEY OUT | verify FILE PUBLIC..."),
    }
    Ok(())
}
