//! The device tokens the launcher keeps for "Stay signed in" (yo3-desktop `multiplayer.rs`): the same store and names,
//! so the helper reads the token direct start resumes and Delete Login Data forgets it. On Windows that is the Credential
//! Manager; under Wine, the prefix's own (the Linux launcher copies a token there when it sets up direct start).

/// The vault's service name: the same as the launcher's.
pub const SERVICE: &str = "yo3-revival.servers";

/// The entry of an account key (Singleplayer's own server, #23), by the server's identity: beside the saved sign-ins
/// (`server:<identity>`), in the same store.
pub fn key_account(identity: &str) -> String { format!("key:{identity}") }

pub trait Vault: Send + Sync {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>>;
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()>;
    fn delete(&self, account: &str) -> anyhow::Result<()>;
}

/// The system's vault.
pub struct SystemVault;

impl Vault for SystemVault {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>> {
        match keyring::Entry::new(SERVICE, account).and_then(|e| e.get_password()) {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            // Backend errors may carry the stored bytes: never in a message.
            Err(_) => anyhow::bail!("the system's credential store can't be read"),
        }
    }
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()> {
        keyring::Entry::new(SERVICE, account).and_then(|e| e.set_password(secret)).map_err(|_| anyhow::anyhow!("the system's credential store can't be written"))
    }
    fn delete(&self, account: &str) -> anyhow::Result<()> {
        match keyring::Entry::new(SERVICE, account).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => anyhow::bail!("the system's credential store can't be written"),
        }
    }
}

/// A vault in memory, for tests.
#[derive(Default)]
pub struct MemoryVault(pub std::sync::Mutex<std::collections::HashMap<String, String>>);

impl Vault for MemoryVault {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>> { Ok(self.0.lock().unwrap().get(account).cloned()) }
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()> { self.0.lock().unwrap().insert(account.into(), secret.into()); Ok(()) }
    fn delete(&self, account: &str) -> anyhow::Result<()> { self.0.lock().unwrap().remove(account); Ok(()) }
}
