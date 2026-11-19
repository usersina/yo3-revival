//! The staff dashboard's second factor (#43): TOTP (RFC 6238: HMAC-SHA1, 6 digits, 30-second steps), its seed sealed at
//! rest, hashed single-use recovery codes, and the throttling and replay protection in front of them.
//!
//! - **Seeds** are 160 random bits, kept in the account's login sealed with AES-256-GCM (`ring`). The key is 32 random
//!   bytes in `data/staff-mfa.key` (created on first use, readable by the host's user only, never part of a backup:
//!   a stolen `server.db` or backup alone does not reveal a seed). Lose the file and every seed is unreadable: the host
//!   runs `yo3-server staff-mfa-reset USER` and the staff enroll again.
//! - **Replay:** a code is accepted for a step newer than the last accepted one, so each step works once (a code from
//!   the step before is accepted too, once, for clock skew).
//! - **Recovery codes** are `new_code()` codes (80 bits), kept as SHA-256 digests, each used once.
//! - **Throttling** reuses the sign-in's [`Throttle`]: a bucket per address and a doubling wait per User ID after a wrong
//!   code, for sign-in codes, step-up codes, enrollment codes and recovery codes alike.
//! - No code, seed or recovery code is ever logged.
use std::net::IpAddr;
use std::path::Path;
use std::time::Instant;

use ring::{aead, hmac};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::api::Api;
use crate::lock::Locked;
use crate::logins::{self, StaffMfa};
use crate::panel::Panel;

/// How long a code stays valid, in seconds.
pub const STEP: i64 = 30;
/// How many recovery codes enrollment (and a regeneration) makes.
pub const RECOVERY_CODES: usize = 10;
/// How long after a second-factor check a sensitive operation is allowed (seconds).
pub const RECENT: i64 = 300;

/// Whether staff must use a second factor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum StaffMfaPolicy {
    /// Every staff account signs in with password and code. A staff account that has not enrolled gets a session that
    /// can enroll and nothing else. The default.
    #[default]
    Required,
    /// A transition setting: an account that has enrolled always needs its code, but one that has not keeps a
    /// password-only session. Public staff access should not stay in this mode.
    Optional,
}

impl StaffMfaPolicy {
    pub fn as_str(self) -> &'static str { match self { Self::Required => "required", Self::Optional => "optional" } }
}

// ---- TOTP ----------------------------------------------------------------------------------------------------------

/// RFC 4648 base32 without padding (what authenticator apps take).
pub fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let (mut out, mut buffer, mut bits) = (String::new(), 0u32, 0u32);
    for b in bytes {
        buffer = buffer << 8 | u32::from(*b);
        bits += 8;
        while bits >= 5 { bits -= 5; out.push(ALPHABET[(buffer >> bits & 31) as usize] as char); }
    }
    if bits > 0 { out.push(ALPHABET[(buffer << (5 - bits) & 31) as usize] as char) }
    out
}

/// The six digits for `step` (RFC 4226 dynamic truncation).
pub fn code_at(seed: &[u8], step: u64) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, seed);
    let tag = hmac::sign(&key, &step.to_be_bytes());
    let tag = tag.as_ref();
    let offset = usize::from(tag[19] & 15);
    let value = u32::from_be_bytes([tag[offset] & 0x7f, tag[offset + 1], tag[offset + 2], tag[offset + 3]]);
    format!("{:06}", value % 1_000_000)
}

/// The step a typed code is valid for at `now`, if it is one newer than `last_step`: the current step, or one either
/// side for a clock that is a little off. Compared in constant time.
pub fn accept(seed: &[u8], code: &str, now: i64, last_step: u64) -> Option<u64> {
    let current = u64::try_from(now / STEP).ok()?;
    let mut found = None;
    for step in [current.saturating_sub(1), current, current + 1] {
        let matches: bool = code_at(seed, step).as_bytes().ct_eq(code.as_bytes()).into();
        if matches && step > last_step && found.is_none_or(|f| step > f) { found = Some(step) }
    }
    found
}

/// What a person typed as a TOTP code: six digits, spaces allowed (authenticator apps show "123 456").
fn totp_digits(typed: &str) -> Option<String> {
    let digits: String = typed.chars().filter(|c| !c.is_whitespace()).collect();
    (digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_digit())).then_some(digits)
}

/// The `otpauth://` link an authenticator app reads from the QR code.
pub fn otpauth_uri(issuer: &str, account: &str, seed: &[u8]) -> String {
    fn encode(text: &str) -> String {
        text.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
    }
    format!("otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30", encode(issuer), encode(account), base32(seed), encode(issuer))
}

// ---- Sealing ---------------------------------------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn unhex(text: &str) -> Option<Vec<u8>> {
    (text.len().is_multiple_of(2) && text.is_ascii()).then(|| (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok()).collect::<Option<Vec<_>>>()).flatten()
}

fn aad(user_id: u32) -> Vec<u8> { [b"yo3-staff-mfa-v1:".as_slice(), &user_id.to_be_bytes()].concat() }

/// Seals a seed for `user_id`: `v1.<nonce>.<ciphertext and tag>` in hex. The account is authenticated data, so a
/// sealed seed copied onto another account does not open.
pub fn seal(key: &[u8; 32], user_id: u32, seed: &[u8]) -> String {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).expect("OS randomness unavailable");
    let sealing = aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_256_GCM, key).expect("a 32-byte key"));
    let mut buffer = seed.to_vec();
    sealing.seal_in_place_append_tag(aead::Nonce::assume_unique_for_key(nonce), aead::Aad::from(aad(user_id)), &mut buffer).expect("sealing a short seed");
    format!("v1.{}.{}", hex(&nonce), hex(&buffer))
}

/// The seed `sealed` holds for `user_id`, if the key is the one that sealed it.
pub fn open(key: &[u8; 32], user_id: u32, sealed: &str) -> Option<Vec<u8>> {
    let mut parts = sealed.split('.');
    (parts.next()? == "v1").then_some(())?;
    let nonce: [u8; 12] = unhex(parts.next()?)?.try_into().ok()?;
    let mut buffer = unhex(parts.next()?)?;
    let opening = aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_256_GCM, key).ok()?);
    let plain = opening.open_in_place(aead::Nonce::assume_unique_for_key(nonce), aead::Aad::from(aad(user_id)), &mut buffer).ok()?;
    Some(plain.to_vec())
}

/// The key file of a data folder: read, or created with the host's user as its only reader.
pub fn load_key(dir: &Path) -> std::io::Result<[u8; 32]> {
    use std::io::Write;
    let path = dir.join("staff-mfa.key");
    let read = || -> std::io::Result<[u8; 32]> {
        unhex(std::fs::read_to_string(&path)?.trim()).and_then(|b| b.try_into().ok())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "staff-mfa.key is not 64 hex digits"))
    };
    if path.exists() { return read() }
    std::fs::create_dir_all(dir)?;
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).expect("OS randomness unavailable");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    match options.open(&path) {
        Ok(mut file) => { file.write_all(hex(&key).as_bytes())?; Ok(key) }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => read(),
        Err(e) => Err(e),
    }
}

// ---- Checking codes ---------------------------------------------------------------------------------------------------

/// Why a code was not accepted.
#[derive(Debug, PartialEq, Eq)]
pub enum CodeError {
    /// Wrong, already used, or for a step too old.
    Invalid,
    /// Too many tries: wait this many seconds.
    Throttled(u64),
    /// The account has no active authenticator, or is gone.
    Unavailable,
    /// The seed could not be opened or the change not saved.
    Failed(String),
}

/// What an accepted code was.
#[derive(Debug, PartialEq, Eq)]
pub enum Used { Totp, Recovery { left: usize } }

enum Found { Step(u64), Recovery(usize) }

/// What `typed` is, checked against an active authenticator at `now` (no change made).
fn evaluate(mfa: &StaffMfa, key: &[u8; 32], user_id: u32, typed: &str, now: i64) -> Result<Found, CodeError> {
    if !mfa.active() { return Err(CodeError::Unavailable) }
    if let Some(digits) = totp_digits(typed) {
        let seed = open(key, user_id, &mfa.seed).ok_or_else(|| CodeError::Failed("the authenticator's seed could not be opened (staff-mfa.key changed?)".into()))?;
        return accept(&seed, &digits, now, mfa.last_step).map(Found::Step).ok_or(CodeError::Invalid)
    }
    let want = logins::digest(&logins::normalize_code(typed));
    if logins::normalize_code(typed).len() != 16 { return Err(CodeError::Invalid) }
    let mut position = None;
    for (i, have) in mfa.recovery.iter().enumerate() { if crate::admin::same(have, &want) { position = Some(i) } }
    position.map(Found::Recovery).ok_or(CodeError::Invalid)
}

fn new_recovery_codes() -> (Vec<String>, Vec<String>) {
    let codes: Vec<String> = (0..RECOVERY_CODES).map(|_| logins::new_code()).collect();
    let digests = codes.iter().map(|c| logins::digest(&logins::normalize_code(c))).collect();
    (codes, digests)
}

impl Panel {
    /// The throttle in front of every code attempt: the address's allowance, and the wait after this User ID's misses.
    fn code_gate(&self, api: &Api, user: &str, peer: IpAddr) -> Result<(), CodeError> {
        let mut throttle = self.mfa_throttle.locked();
        throttle.set_limits(api.settings.now().limits);
        let at = Instant::now();
        match throttle.address(peer, at).err().or_else(|| throttle.user_wait(user, at)) {
            Some(wait) => Err(CodeError::Throttled(wait.as_secs().max(1))),
            None => Ok(()),
        }
    }

    fn code_missed(&self, user: &str) { self.mfa_throttle.locked().failed(user, Instant::now()) }
    fn code_accepted(&self, user: &str) { self.mfa_throttle.locked().succeeded(user) }

    /// Checks and uses a TOTP or recovery code of `user_id`: a step or a recovery code works once.
    pub(crate) fn use_code(&self, api: &Api, user_id: u32, peer: IpAddr, typed: &str) -> Result<Used, CodeError> {
        let user = api.accounts.locked().login_by_id(user_id).map(|l| l.user.clone()).ok_or(CodeError::Unavailable)?;
        self.code_gate(api, &user, peer)?;
        let key = api.mfa_key().map_err(CodeError::Failed)?;
        let now = logins::now();
        let check = |login: &logins::Login| login.mfa.as_ref().ok_or(CodeError::Unavailable).and_then(|m| evaluate(m, &key, user_id, typed, now));
        let found = { let d = api.accounts.locked(); d.login_by_id(user_id).ok_or(CodeError::Unavailable).and_then(check) };
        let _found = match found {
            Ok(found) => found,
            Err(CodeError::Invalid) => { self.code_missed(&user); tracing::warn!("[staff-mfa] a code was refused"); return Err(CodeError::Invalid) }
            Err(e) => return Err(e),
        };
        // Committed against the login as it is now: another request may have used this step a moment ago.
        let used = api.accounts.locked().update_login(user_id, |login| {
            let mfa = login.mfa.as_mut().ok_or("unavailable")?;
            match evaluate(mfa, &key, user_id, typed, now).map_err(|e| if e == CodeError::Invalid { "invalid".to_string() } else { "unavailable".to_string() })? {
                Found::Step(step) => { mfa.last_step = step; Ok(Used::Totp) }
                Found::Recovery(at) => { mfa.recovery.remove(at); Ok(Used::Recovery { left: mfa.recovery.len() }) }
            }
        });
        match used {
            Ok(used) => { self.code_accepted(&user); Ok(used) }
            Err(e) if e == "invalid" => { self.code_missed(&user); Err(CodeError::Invalid) }
            Err(e) if e == "unavailable" => Err(CodeError::Unavailable),
            Err(e) => Err(CodeError::Failed(e)),
        }
    }

    /// Starts (or restarts) enrollment: a fresh seed, sealed as pending until a valid code confirms it. The seed and
    /// its `otpauth://` link, shown to the account's owner only.
    pub(crate) fn begin_enrollment(&self, api: &Api, user_id: u32) -> Result<(String, String, String), String> {
        let key = api.mfa_key()?;
        let mut seed = [0u8; 20];
        getrandom::fill(&mut seed).map_err(|e| e.to_string())?;
        let sealed = seal(&key, user_id, &seed);
        let account = api.accounts.locked().update_login(user_id, |login| {
            login.mfa.get_or_insert_with(StaffMfa::default).pending = sealed;
            Ok(login.user.clone())
        })?;
        let issuer = "YO3 Revival";
        Ok((base32(&seed), otpauth_uri(issuer, &account, &seed), account))
    }

    /// Confirms enrollment with a code from the pending seed: it becomes the active seed, recovery codes are made, and
    /// every session of the account but the caller's ends (the epoch moves). The recovery codes, in the clear once.
    pub(crate) fn confirm_enrollment(&self, api: &Api, user_id: u32, peer: IpAddr, typed: &str) -> Result<Vec<String>, CodeError> {
        let (user, pending) = {
            let d = api.accounts.locked();
            let login = d.login_by_id(user_id).ok_or(CodeError::Unavailable)?;
            (login.user.clone(), login.mfa.as_ref().map(|m| m.pending.clone()).filter(|p| !p.is_empty()).ok_or(CodeError::Unavailable)?)
        };
        self.code_gate(api, &user, peer)?;
        let key = api.mfa_key().map_err(CodeError::Failed)?;
        let seed = open(&key, user_id, &pending).ok_or_else(|| CodeError::Failed("the pending seed could not be opened".into()))?;
        let digits = totp_digits(typed).ok_or(CodeError::Invalid);
        let last = api.accounts.locked().login_by_id(user_id).and_then(|l| l.mfa.as_ref().map(|m| m.last_step)).unwrap_or(0);
        let step = digits.and_then(|d| accept(&seed, &d, logins::now(), last).ok_or(CodeError::Invalid));
        let Ok(step) = step else { self.code_missed(&user); tracing::warn!("[staff-mfa] an enrollment code was refused"); return Err(CodeError::Invalid) };
        let (codes, digests) = new_recovery_codes();
        api.accounts.locked().update_login(user_id, |login| {
            let mfa = login.mfa.as_mut().filter(|m| m.pending == pending).ok_or("enrollment changed")?;
            mfa.seed = std::mem::take(&mut mfa.pending);
            mfa.last_step = step;
            mfa.recovery = digests;
            mfa.enrolled_at = logins::now();
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?;
            Ok(())
        }).map_err(CodeError::Failed)?;
        self.code_accepted(&user);
        Ok(codes)
    }

    /// Replaces every recovery code; the new ones, in the clear once.
    pub(crate) fn regenerate_recovery(&self, api: &Api, user_id: u32) -> Result<Vec<String>, String> {
        let (codes, digests) = new_recovery_codes();
        api.accounts.locked().update_login(user_id, |login| {
            let mfa = login.mfa.as_mut().filter(|m| m.active()).ok_or("no active authenticator")?;
            mfa.recovery = digests;
            Ok(())
        })?;
        Ok(codes)
    }
}

impl Api {
    /// The key that seals staff seeds: `data/staff-mfa.key`, read or created at first use (a server with no data folder
    /// holds a key for its own lifetime, for tests).
    pub(crate) fn mfa_key(&self) -> Result<[u8; 32], String> {
        let mut held = self.mfa_key.locked();
        if let Some(key) = *held { return Ok(key) }
        let key = match &self.data_dir {
            Some(dir) => load_key(dir).map_err(|e| format!("staff-mfa.key: {e}"))?,
            None => { let mut key = [0u8; 32]; getrandom::fill(&mut key).map_err(|e| e.to_string())?; key }
        };
        *held = Some(key);
        Ok(key)
    }

    /// Local recovery (`yo3-server staff-mfa-reset USER`): the account's authenticator and recovery codes are removed and
    /// every staff session of it ends. Its next sign-in enrolls again (Required) or goes on password-only (Optional).
    pub(crate) fn reset_staff_mfa(&self, user: &str) -> Result<serde_json::Value, String> {
        let mut d = self.accounts.locked();
        let (user_id, login) = d.login_of(user).ok_or("no such User ID")?;
        let had = login.mfa.is_some();
        d.update_login(user_id, |login| {
            login.mfa = None;
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?;
            Ok(())
        })?;
        Ok(serde_json::json!({ "user": login.user, "had_authenticator": had,
            "message": format!("{}'s authenticator and recovery codes are removed and its staff sessions ended.", login.user) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_6238_sha1_vectors() {
        let seed = b"12345678901234567890";
        for (time, code) in [(59, "287082"), (1_111_111_109, "081804"), (1_111_111_111, "050471"), (1_234_567_890, "005924"), (2_000_000_000, "279037")] {
            assert_eq!(code_at(seed, (time / STEP) as u64), code, "T={time}");
        }
    }

    #[test]
    fn base32_matches_rfc_4648() {
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
        assert_eq!(base32(b"fooba"), "MZXW6YTB");
        assert_eq!(base32(b"f"), "MY");
        assert_eq!(base32(b""), "");
    }

    #[test]
    fn a_step_works_once_and_old_steps_never() {
        let seed = b"12345678901234567890";
        let now = 1_000_000_000;
        let step = (now / STEP) as u64;
        let code = code_at(seed, step);
        assert_eq!(accept(seed, &code, now, 0), Some(step));
        assert_eq!(accept(seed, &code, now, step), None, "replayed");
        assert_eq!(accept(seed, &code_at(seed, step - 1), now, step), None, "an older step after a newer one");
        assert_eq!(accept(seed, &code_at(seed, step + 1), now, step), Some(step + 1), "a little ahead is skew");
        assert_eq!(accept(seed, &code_at(seed, step - 2), now, 0), None, "too old");
        assert_eq!(accept(seed, "12345", now, 0), None);
    }

    #[test]
    fn a_sealed_seed_opens_only_for_its_account_and_key() {
        let key = [7u8; 32];
        let sealed = seal(&key, 1001, b"twenty bytes of seed");
        assert!(sealed.starts_with("v1.") && !sealed.contains("twenty"));
        assert_eq!(open(&key, 1001, &sealed).as_deref(), Some(b"twenty bytes of seed".as_slice()));
        assert!(open(&key, 1002, &sealed).is_none(), "another account");
        assert!(open(&[8u8; 32], 1001, &sealed).is_none(), "another key");
        assert!(open(&key, 1001, &sealed.replace(".", "x")).is_none());
        assert_ne!(seal(&key, 1001, b"twenty bytes of seed"), sealed, "a fresh nonce each time");
    }

    #[test]
    fn the_otpauth_link_names_the_account_and_issuer() {
        let uri = otpauth_uri("YO3 Revival", "Yugi M", b"12345678901234567890");
        assert_eq!(uri, "otpauth://totp/YO3%20Revival:Yugi%20M?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=YO3%20Revival&algorithm=SHA1&digits=6&period=30");
    }

    #[test]
    fn the_key_file_is_made_once_and_private() {
        let dir = std::env::temp_dir().join(format!("yo3-mfa-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = load_key(&dir).unwrap();
        assert_eq!(load_key(&dir).unwrap(), first);
        #[cfg(unix)] assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(dir.join("staff-mfa.key")).unwrap().permissions()) & 0o777, 0o600);
        std::fs::write(dir.join("staff-mfa.key"), "not hex").unwrap();
        assert!(load_key(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
