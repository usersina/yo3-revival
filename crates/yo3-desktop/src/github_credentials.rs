//! GitHub credentials belong exclusively in the operating system's secure vault.
//!
//! Callers may keep a session in memory if the vault is unavailable. This module
//! never falls back to application preferences, SQLite, or a plaintext file.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "yo3-revival.github";

// Deliberately no Debug: event/error logging must not format either token.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoredCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub refresh_expires_at: Option<u64>,
    pub login: String,
}

pub fn load(client_id: &str) -> Result<Option<StoredCredential>> {
    load_from(&NativeVault, client_id)
}

pub fn save(client_id: &str, credential: &StoredCredential) -> Result<()> {
    save_to(&NativeVault, client_id, credential)
}

pub fn clear(client_id: &str) -> Result<()> {
    clear_from(&NativeVault, client_id)
}

trait Vault {
    fn get(&self, account: &str) -> keyring::Result<String>;
    fn set(&self, account: &str, secret: &str) -> keyring::Result<()>;
    fn delete(&self, account: &str) -> keyring::Result<()>;
}

struct NativeVault;

impl Vault for NativeVault {
    fn get(&self, account: &str) -> keyring::Result<String> {
        keyring::Entry::new(SERVICE, account)?.get_password()
    }

    fn set(&self, account: &str, secret: &str) -> keyring::Result<()> {
        keyring::Entry::new(SERVICE, account)?.set_password(secret)
    }

    fn delete(&self, account: &str) -> keyring::Result<()> {
        keyring::Entry::new(SERVICE, account)?.delete_credential()
    }
}

fn load_from(vault: &impl Vault, client_id: &str) -> Result<Option<StoredCredential>> {
    validate_client_id(client_id)?;
    let encoded = match vault.get(client_id) {
        Ok(encoded) => encoded,
        Err(keyring::Error::NoEntry) => return Ok(None),
        // Neither backend errors nor serde errors may enter an anyhow source
        // chain: they can contain the stored bytes or offending JSON value.
        Err(_) => return Err(anyhow!("Could not read GitHub sign-in from the system credential store")),
    };
    let credential: StoredCredential = serde_json::from_str(&encoded)
        .map_err(|_| anyhow!("Saved GitHub sign-in is invalid; sign in again"))?;
    validate_credential(&credential)?;
    Ok(Some(credential))
}

fn save_to(vault: &impl Vault, client_id: &str, credential: &StoredCredential) -> Result<()> {
    validate_client_id(client_id)?;
    validate_credential(credential)?;
    let encoded = serde_json::to_string(credential)
        .map_err(|_| anyhow!("Could not encode GitHub sign-in for secure storage"))?;
    // One replacement keeps the rotated access and refresh tokens together.
    vault.set(client_id, &encoded)
        .map_err(|_| anyhow!("Could not save GitHub sign-in in the system credential store"))
}

fn clear_from(vault: &impl Vault, client_id: &str) -> Result<()> {
    validate_client_id(client_id)?;
    match vault.delete(client_id) {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(anyhow!("Could not remove GitHub sign-in from the system credential store")),
    }
}

fn validate_client_id(client_id: &str) -> Result<()> {
    if client_id.is_empty() || client_id.len() > 200
        || !client_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(anyhow!("GitHub application client ID is invalid"));
    }
    Ok(())
}

fn validate_credential(credential: &StoredCredential) -> Result<()> {
    let valid_token = |token: &str| !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic());
    if !valid_token(&credential.access_token)
        || credential.refresh_token.as_deref().is_some_and(|token| !valid_token(token))
        || credential.login.trim().is_empty()
        || credential.login.chars().any(char::is_control)
    {
        return Err(anyhow!("Saved GitHub sign-in is invalid; sign in again"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};

    #[derive(Default)]
    struct MockVault {
        entries: RefCell<HashMap<String, String>>,
        fail: bool,
    }

    impl MockVault {
        fn check(&self) -> keyring::Result<()> {
            if self.fail {
                // A lower-level error can contain raw secret bytes. Never attach it.
                Err(keyring::Error::BadEncoding(b"ghu_secret_refresh_ghr_secret".to_vec()))
            } else {
                Ok(())
            }
        }
    }

    impl Vault for MockVault {
        fn get(&self, account: &str) -> keyring::Result<String> {
            self.check()?;
            self.entries.borrow().get(account).cloned().ok_or(keyring::Error::NoEntry)
        }

        fn set(&self, account: &str, secret: &str) -> keyring::Result<()> {
            self.check()?;
            self.entries.borrow_mut().insert(account.to_owned(), secret.to_owned());
            Ok(())
        }

        fn delete(&self, account: &str) -> keyring::Result<()> {
            self.check()?;
            self.entries.borrow_mut().remove(account).map(|_| ()).ok_or(keyring::Error::NoEntry)
        }
    }

    fn credential() -> StoredCredential {
        StoredCredential {
            access_token: "ghu_secret".into(),
            refresh_token: Some("ghr_secret".into()),
            expires_at: Some(1_800_000_000),
            refresh_expires_at: Some(1_815_897_600),
            login: "test-player".into(),
        }
    }

    fn assert_safe_error<T>(result: Result<T>) {
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("Expected a safe credential error"),
        };
        let rendered = format!("{error} {error:#} {error:?}");
        for secret in ["ghu_secret", "ghr_secret", "private-client-value"] {
            assert!(!rendered.contains(secret), "Credential error leaked protected data");
        }
        assert!(error.source().is_none(), "Credential errors must not retain unsafe sources");
    }

    #[test]
    fn round_trip_preserves_tokens_identity_and_expirations() {
        let vault = MockVault::default();
        save_to(&vault, "Iv1.test-app", &credential()).unwrap();
        let loaded = load_from(&vault, "Iv1.test-app").unwrap().unwrap();
        assert_eq!(loaded.access_token, "ghu_secret");
        assert_eq!(loaded.refresh_token.as_deref(), Some("ghr_secret"));
        assert_eq!(loaded.expires_at, Some(1_800_000_000));
        assert_eq!(loaded.refresh_expires_at, Some(1_815_897_600));
        assert_eq!(loaded.login, "test-player");
        assert_eq!(vault.entries.borrow().len(), 1);
    }

    #[test]
    fn nonexpiring_credentials_round_trip_without_refresh_fields() {
        let vault = MockVault::default();
        let mut stored = credential();
        stored.refresh_token = None;
        stored.expires_at = None;
        stored.refresh_expires_at = None;
        save_to(&vault, "Iv1.test-app", &stored).unwrap();
        let loaded = load_from(&vault, "Iv1.test-app").unwrap().unwrap();
        assert!(loaded.refresh_token.is_none());
        assert!(loaded.expires_at.is_none());
        assert!(loaded.refresh_expires_at.is_none());
    }

    #[test]
    fn absent_credentials_and_repeated_clear_are_successful() {
        let vault = MockVault::default();
        assert!(load_from(&vault, "Iv1.test-app").unwrap().is_none());
        clear_from(&vault, "Iv1.test-app").unwrap();
        save_to(&vault, "Iv1.test-app", &credential()).unwrap();
        clear_from(&vault, "Iv1.test-app").unwrap();
        clear_from(&vault, "Iv1.test-app").unwrap();
        assert!(load_from(&vault, "Iv1.test-app").unwrap().is_none());
    }

    #[test]
    fn client_ids_have_independent_entries_and_clear_scope() {
        let vault = MockVault::default();
        save_to(&vault, "Iv1.app-a", &credential()).unwrap();
        let mut other = credential();
        other.access_token = "ghu_other".into();
        save_to(&vault, "Iv1.app-b", &other).unwrap();
        clear_from(&vault, "Iv1.app-a").unwrap();
        assert!(load_from(&vault, "Iv1.app-a").unwrap().is_none());
        assert_eq!(load_from(&vault, "Iv1.app-b").unwrap().unwrap().access_token, "ghu_other");
    }

    #[test]
    fn refresh_replaces_access_and_refresh_tokens_in_one_entry() {
        let vault = MockVault::default();
        save_to(&vault, "Iv1.test-app", &credential()).unwrap();
        let mut rotated = credential();
        rotated.access_token = "ghu_rotated".into();
        rotated.refresh_token = Some("ghr_rotated".into());
        save_to(&vault, "Iv1.test-app", &rotated).unwrap();
        let loaded = load_from(&vault, "Iv1.test-app").unwrap().unwrap();
        assert_eq!(loaded.access_token, "ghu_rotated");
        assert_eq!(loaded.refresh_token.as_deref(), Some("ghr_rotated"));
        assert_eq!(vault.entries.borrow().len(), 1);
    }

    #[test]
    fn unavailable_vault_errors_never_retain_backend_secrets() {
        let vault = MockVault { fail: true, ..MockVault::default() };
        assert_safe_error(load_from(&vault, "Iv1.test-app"));
        assert_safe_error(save_to(&vault, "Iv1.test-app", &credential()));
        assert_safe_error(clear_from(&vault, "Iv1.test-app"));
        assert!(vault.entries.borrow().is_empty());
    }

    #[test]
    fn malformed_cached_data_has_safe_errors() {
        let vault = MockVault::default();
        for raw in [
            "{\"ghu_secret\": \"ghr_secret\"}",
            "{\"access_token\":\"ghu_secret\",\"login\":\"test-player\",\"expires_at\":\"ghr_secret\"}",
            "ghu_secret ghr_secret",
            "null",
        ] {
            vault.entries.borrow_mut().insert("Iv1.test-app".into(), raw.into());
            assert_safe_error(load_from(&vault, "Iv1.test-app"));
        }
    }

    #[test]
    fn invalid_credentials_do_not_overwrite_valid_saved_session() {
        let vault = MockVault::default();
        save_to(&vault, "Iv1.test-app", &credential()).unwrap();
        for invalid in ["", "\nghu_secret", "ghu_secret\r\n"] {
            let mut stored = credential();
            stored.access_token = invalid.into();
            assert_safe_error(save_to(&vault, "Iv1.test-app", &stored));
        }
        assert_eq!(load_from(&vault, "Iv1.test-app").unwrap().unwrap().access_token, "ghu_secret");
    }

    #[test]
    fn invalid_client_ids_fail_before_touching_vault() {
        let vault = MockVault::default();
        for invalid in ["", "private-client-value\n", "private-client-value\0"] {
            assert_safe_error(load_from(&vault, invalid));
            assert_safe_error(save_to(&vault, invalid, &credential()));
            assert_safe_error(clear_from(&vault, invalid));
        }
        assert!(vault.entries.borrow().is_empty());
    }

    #[test]
    fn credential_cannot_be_formatted_by_debug_logging() {
        // Ambiguous trait resolution deliberately makes adding Debug fail to compile.
        trait NotDebug<A> { fn check() {} }
        impl<T: ?Sized> NotDebug<()> for T {}
        impl<T: ?Sized + std::fmt::Debug> NotDebug<u8> for T {}
        let _ = <StoredCredential as NotDebug<_>>::check;
    }
}
