//! Short-lived, single-use transport tickets. Credentials never enter game packet logs.
use std::time::{Duration, Instant};
use std::collections::HashMap;
use yo3_tunnel::{Role, Token};

pub const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
pub const TICKET_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct TlsConfig { pub port: u16, pub certificate: std::path::PathBuf, pub private_key: std::path::PathBuf }

/// Generate the host's public certificate and private key once. Existing files are never overwritten.
pub fn initialize(dir: &std::path::Path, names: Vec<String>) -> anyhow::Result<TlsConfig> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let certificate = dir.join("server.crt");
    let private_key = dir.join("server.key");
    anyhow::ensure!(!certificate.exists() && !private_key.exists(), "certificate or key already exists; keep the existing server identity");
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let signing_key = rcgen::KeyPair::generate()?;
    let cert = certificate_parameters(names)?.self_signed(&signing_key)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut key = options.open(&private_key)?;
    key.write_all(signing_key.serialize_pem().as_bytes())?;
    let result = std::fs::OpenOptions::new().write(true).create_new(true).open(&certificate)
        .and_then(|mut f| f.write_all(cert.pem().as_bytes()));
    if let Err(e) = result { let _ = std::fs::remove_file(&private_key); return Err(e.into()) }
    Ok(TlsConfig { port: 7443, certificate, private_key })
}

/// How far behind the host's clock a client's may be and still accept a new or renewed identity.
const CLOCK_SLACK: std::time::Duration = std::time::Duration::from_secs(86400);

fn certificate_parameters(names: Vec<String>) -> anyhow::Result<rcgen::CertificateParams> {
    let mut params = rcgen::CertificateParams::new(names)?;
    // Backdated a day: a friend whose clock is behind (a wrong time zone is up to 14 hours) still accepts a server just
    // set up. Clients pin the key, so the earlier start costs nothing.
    let now = std::time::SystemTime::now();
    params.not_before = (now - CLOCK_SLACK).into();
    params.not_after = (now + std::time::Duration::from_secs(365 * 86400)).into();
    Ok(params)
}

/// The GUI calls this only while stopped. Renew self-managed identities near expiry or when names change,
/// preserving the private key and previous names. An externally issued certificate is never replaced here.
pub fn maintain_identity(dir: &std::path::Path, mut names: Vec<String>) -> anyhow::Result<TlsConfig> {
    let certificate = dir.join("server.crt"); let private_key = dir.join("server.key");
    if !certificate.exists() && !private_key.exists() { return initialize(dir, names) }
    anyhow::ensure!(certificate.is_file() && private_key.is_file(), "Server identity is incomplete. Restore its matching certificate/key backup; a replacement key was not created.");
    let bytes = std::fs::read(&certificate)?; anyhow::ensure!(bytes.len() <= 65536, "Server certificate is unexpectedly large");
    let (_, pem) = x509_parser::pem::parse_x509_pem(&bytes).map_err(|_| anyhow::anyhow!("Server certificate is invalid; restore or repair it before starting"))?;
    let (_, parsed) = x509_parser::parse_x509_certificate(&pem.contents).map_err(|_| anyhow::anyhow!("Invalid server identity certificate"))?;
    let mut existing = Vec::new();
    if let Some(san) = parsed.subject_alternative_name()? { for name in &san.value.general_names {
        match name {
            x509_parser::extensions::GeneralName::DNSName(name) => existing.push(name.to_string()),
            x509_parser::extensions::GeneralName::IPAddress(bytes) if bytes.len() == 4 => existing.push(std::net::Ipv4Addr::from(<[u8; 4]>::try_from(*bytes)?).to_string()),
            x509_parser::extensions::GeneralName::IPAddress(bytes) if bytes.len() == 16 => existing.push(std::net::Ipv6Addr::from(<[u8; 16]>::try_from(*bytes)?).to_string()),
            _ => {},
        }
    } }
    let now = chrono::Utc::now().timestamp();
    let needs_names = names.iter().any(|name| !existing.contains(name));
    let needs_renewal = parsed.validity().not_after.timestamp() < now + 30 * 86400 || parsed.validity().not_before.timestamp() > now;
    if needs_names || needs_renewal {
        anyhow::ensure!(parsed.issuer() == parsed.subject(), "The externally issued certificate needs names/renewal from its issuer; it was not replaced with a self-signed identity.");
        names.extend(existing); names.sort(); names.dedup(); anyhow::ensure!(names.len() <= 64, "Too many names on the server identity");
        reissue_certificate(dir, names)?;
    }
    Ok(TlsConfig { port: 7443, certificate, private_key })
}

/// At a server's start: renew the data folder's own identity when it is within 30 days of expiry (same key and names,
/// so pinned clients keep working). Before this only the Desktop app renewed, and a command-line host's clients all
/// stopped connecting a year after `init-auth`. Nothing happens without an identity, and an externally issued
/// certificate is left to its issuer (a warning). Whether the certificate was renewed.
pub fn renew_on_start(dir: &std::path::Path) -> bool {
    if !dir.join("server.crt").is_file() || !dir.join("server.key").is_file() { return false }
    let before = std::fs::read(dir.join("server.crt")).ok();
    match maintain_identity(dir, vec![]) {
        Ok(_) => {
            let renewed = std::fs::read(dir.join("server.crt")).ok() != before;
            if renewed { tracing::info!("server certificate renewed for another year (same key: launchers that trust it keep working)"); }
            renewed
        }
        Err(e) => { tracing::warn!("server certificate not renewed: {e:#}"); false }
    }
}

/// Explicitly update the names on an existing identity. The private key and account data are untouched.
/// Call only while the server is stopped; the previous public certificate is kept for recovery.
pub fn reissue_certificate(dir: &std::path::Path, names: Vec<String>) -> anyhow::Result<std::path::PathBuf> {
    let key = rcgen::KeyPair::from_pem(&std::fs::read_to_string(dir.join("server.key"))?)?;
    let params = certificate_parameters(names)?;
    let certificate = params.self_signed(&key)?.pem();
    let backup = dir.join("backups").join(format!("certificate-{}.crt", chrono::Utc::now().format("%Y%m%d-%H%M%S%.9f")));
    std::fs::create_dir_all(backup.parent().unwrap())?;
    std::fs::copy(dir.join("server.crt"), &backup)?;
    let temporary = dir.join("server.crt.next");
    std::fs::write(&temporary, certificate)?;
    std::fs::rename(&temporary, dir.join("server.crt"))?;
    Ok(backup)
}

/// Isolated, generated credentials for the in-process integration-test server.
pub(crate) fn test_config(log_dir: &std::path::Path) -> TlsConfig {
    let dir = log_dir.join("test-auth");
    let mut tls = if dir.join("server.crt").exists() {
        TlsConfig { port: 0, certificate: dir.join("server.crt"), private_key: dir.join("server.key") }
    } else { initialize(&dir, vec!["localhost".into()]).expect("test certificate") };
    tls.port = 0;
    crate::store::initialize(&dir.join("data")).expect("test database");
    let mut d = crate::accounts::Directory::open(vec!["test".into()], &dir.join("data/accounts.json")).expect("test accounts");
    if !dir.join("client.key").exists() {
        let key = d.add_account(&["test".into()]).expect("test account");
        std::fs::write(dir.join("client.key"), key).expect("test credential");
    }
    tls
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity { pub user_id: u32, pub session_id: u64 }

#[derive(Default)]
pub struct Sessions {
    sessions: HashMap<Token, Session>,
    tickets: HashMap<Token, Ticket>,
    next_id: u64,
}

struct Session { identity: Identity, expires: Instant, revoked: tokio::sync::watch::Sender<bool> }
struct Ticket { session: Token, role: Role, expires: Instant }

fn random_token() -> Token {
    let mut token = [0; 32];
    getrandom::fill(&mut token).expect("OS randomness unavailable");
    token
}

impl Sessions {
    pub fn logout(&mut self, session: &Token) {
        self.sessions.remove(session);
        self.tickets.retain(|_, t| &t.session != session);
    }

    /// Ends every session of `user_id` (Sign out everywhere): their games are disconnected.
    pub fn logout_user(&mut self, user_id: u32) {
        let theirs: Vec<Token> = self.sessions.iter().filter(|(_, s)| s.identity.user_id == user_id).map(|(t, _)| *t).collect();
        for token in theirs { self.logout(&token) }
    }

    /// The users signed in now (each counted once).
    pub fn users(&self) -> usize {
        self.sessions.values().map(|s| s.identity.user_id).collect::<std::collections::HashSet<_>>().len()
    }

    /// Whether `user_id` is signed in now (signing in again replaces that session: the newest wins).
    pub fn signed_in(&self, user_id: u32) -> bool { self.sessions.values().any(|s| s.identity.user_id == user_id) }

    /// Ends every session whose user `end` picks (maintenance: everyone but the staff).
    pub fn logout_where(&mut self, end: impl Fn(u32) -> bool) {
        let theirs: Vec<Token> = self.sessions.iter().filter(|(_, s)| end(s.identity.user_id)).map(|(t, _)| *t).collect();
        for token in theirs { self.logout(&token) }
    }

    pub fn lease(&self, identity: Identity) -> Option<tokio::sync::watch::Receiver<bool>> {
        self.sessions.values().find(|s| s.identity == identity).map(|s| s.revoked.subscribe())
    }
    fn prune(&mut self, now: Instant) {
        self.sessions.retain(|_, s| now < s.expires);
        self.tickets.retain(|_, t| now < t.expires && self.sessions.contains_key(&t.session));
    }

    /// A new session for `user_id`. The newest login wins (D12): the account's older sessions end now, which closes
    /// their connections (the game shows its own disconnect dialog), so a crashed client never locks its owner out.
    pub fn login(&mut self, user_id: u32, now: Instant) -> Option<Token> {
        self.prune(now);
        self.logout_user(user_id);
        // A hard ceiling only: the `max-players` setting refuses well before it (#27).
        if self.sessions.len() >= 100_000 { return None }
        self.next_id = self.next_id.checked_add(1)?;
        let token = random_token();
        let (revoked, _) = tokio::sync::watch::channel(false);
        self.sessions.insert(token, Session { identity: Identity { user_id, session_id: self.next_id }, expires: now + SESSION_TTL, revoked });
        Some(token)
    }

    pub fn ticket(&mut self, session: &Token, role: Role, now: Instant) -> Option<Token> {
        self.prune(now);
        if !self.sessions.contains_key(session) || self.tickets.len() >= 4096 { return None }
        let token = random_token();
        self.tickets.insert(token, Ticket { session: *session, role, expires: now + TICKET_TTL });
        Some(token)
    }

    pub fn consume(&mut self, ticket: &Token, role: Role, now: Instant) -> Option<(Identity, Instant)> {
        let ticket = self.tickets.remove(ticket)?;
        let session = self.sessions.get(&ticket.session)?;
        if ticket.role != role || now >= ticket.expires || now >= session.expires { return None }
        Some((session.identity, session.expires))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_identity_renewal_preserves_the_key_and_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("yo3-auto-renew-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tls = initialize(&dir, vec!["localhost".into()]).unwrap();
        let private = std::fs::read(&tls.private_key).unwrap();
        let identity = yo3_tunnel::identity_from_pem(&std::fs::read(&tls.certificate).unwrap()).unwrap();
        let key = rcgen::KeyPair::from_pem(std::str::from_utf8(&private).unwrap()).unwrap();
        let mut expired = rcgen::CertificateParams::new(vec!["localhost".into()]).unwrap();
        expired.not_before = rcgen::date_time_ymd(2000, 1, 1); expired.not_after = rcgen::date_time_ymd(2001, 1, 1);
        std::fs::write(&tls.certificate, expired.self_signed(&key).unwrap().pem()).unwrap();
        maintain_identity(&dir, vec!["localhost".into(), "friends.example".into()]).unwrap();
        assert_eq!(std::fs::read(&tls.private_key).unwrap(), private);
        let renewed = std::fs::read(&tls.certificate).unwrap();
        assert_eq!(yo3_tunnel::identity_from_pem(&renewed).unwrap(), identity);
        maintain_identity(&dir, vec!["localhost".into(), "friends.example".into()]).unwrap();
        assert_eq!(std::fs::read(&tls.certificate).unwrap(), renewed, "a healthy identity is not repeatedly reissued");
        std::fs::write(&tls.certificate, "broken certificate").unwrap();
        assert!(maintain_identity(&dir, vec!["localhost".into()]).is_err());
        assert_eq!(std::fs::read(&tls.private_key).unwrap(), private);
        std::fs::remove_dir_all(dir).unwrap();
    }
    /// Host for friends (#37) on a PC whose clock runs 9 hours ahead of a friend's (a wrong time zone): the friend
    /// refused the new server as "not yet valid". A new identity is valid from a day before its host's clock.
    #[test]
    fn a_new_identity_is_valid_for_a_client_whose_clock_is_behind() {
        let dir = std::env::temp_dir().join(format!("yo3-clock-slack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tls = initialize(&dir, vec!["localhost".into()]).unwrap();
        let bytes = std::fs::read(&tls.certificate).unwrap();
        let (_, pem) = x509_parser::pem::parse_x509_pem(&bytes).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&pem.contents).unwrap();
        let now = chrono::Utc::now().timestamp();
        let behind = x509_parser::time::ASN1Time::from_timestamp(now - 14 * 3600).unwrap();
        assert!(cert.validity().is_valid_at(behind), "valid for a clock 14 hours behind: {:?}", cert.validity());
        assert!(cert.validity().not_after.timestamp() >= now + 365 * 86400 - 60, "still a year ahead");
        assert!(!renew_on_start(&dir), "a fresh backdated certificate is kept");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn a_server_start_renews_an_expiring_identity_with_the_same_key_and_names() {
        let dir = std::env::temp_dir().join(format!("yo3-renew-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!renew_on_start(&dir), "no identity: nothing is created");
        assert!(!dir.join("server.key").exists());
        let tls = initialize(&dir, vec!["localhost".into(), "friends.example".into()]).unwrap();
        assert!(!renew_on_start(&dir), "a fresh certificate is kept");
        let key_pem = std::fs::read_to_string(&tls.private_key).unwrap();
        let identity = yo3_tunnel::identity_from_pem(&std::fs::read(&tls.certificate).unwrap()).unwrap();
        let key = rcgen::KeyPair::from_pem(&key_pem).unwrap();
        let mut old = rcgen::CertificateParams::new(vec!["localhost".into(), "friends.example".into()]).unwrap();
        old.not_before = rcgen::date_time_ymd(2000, 1, 1); old.not_after = rcgen::date_time_ymd(2001, 1, 1);
        std::fs::write(&tls.certificate, old.self_signed(&key).unwrap().pem()).unwrap();
        assert!(renew_on_start(&dir));
        assert_eq!(std::fs::read_to_string(&tls.private_key).unwrap(), key_pem);
        let renewed = std::fs::read(&tls.certificate).unwrap();
        assert_eq!(yo3_tunnel::identity_from_pem(&renewed).unwrap(), identity, "pinned clients still match");
        let (_, pem) = x509_parser::pem::parse_x509_pem(&renewed).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&pem.contents).unwrap();
        assert!(cert.validity().not_after.timestamp() > chrono::Utc::now().timestamp() + 300 * 86400);
        let names = format!("{:?}", cert.subject_alternative_name().unwrap().unwrap().value);
        assert!(names.contains("friends.example"), "{names}");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn updating_host_names_preserves_the_private_key_and_the_previous_certificate() {
        let dir = std::env::temp_dir().join(format!("yo3-renew-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tls = initialize(&dir, vec!["localhost".into()]).unwrap();
        let key = std::fs::read(&tls.private_key).unwrap();
        let original = std::fs::read(&tls.certificate).unwrap();
        let backup = reissue_certificate(&dir, vec!["localhost".into(), "friends.example".into()]).unwrap();
        assert_eq!(std::fs::read(&tls.private_key).unwrap(), key);
        assert_eq!(std::fs::read(backup).unwrap(), original);
        assert_ne!(std::fs::read(&tls.certificate).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn initializing_auth_protects_the_whole_credential_directory_and_existing_backups() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("yo3-private-auth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("backups/old")).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.join("backups/old/accounts.json"), b"existing account data").unwrap();
        let tls = initialize(&dir, vec!["localhost".into()]).unwrap();
        assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        let key = std::fs::read(&tls.private_key).unwrap();
        assert!(initialize(&dir, vec!["other.example".into()]).is_err());
        assert_eq!(std::fs::read(tls.private_key).unwrap(), key, "server identity not replaced");
        assert_eq!(std::fs::read(dir.join("backups/old/accounts.json")).unwrap(), b"existing account data");
    }

    #[test]
    fn tickets_are_single_use_and_scoped_to_the_connection_role() {
        let mut sessions = Sessions::default();
        let now = Instant::now();
        let login = sessions.login(1001, now).unwrap();
        let front = sessions.ticket(&login, Role::Front, now).unwrap();
        let (identity, _) = sessions.consume(&front, Role::Front, now).unwrap();
        assert_eq!(identity.user_id, 1001);
        assert!(sessions.consume(&front, Role::Front, now).is_none(), "replay");
        let wrong = sessions.ticket(&login, Role::Front, now).unwrap();
        assert!(sessions.consume(&wrong, Role::Duel, now).is_none(), "role confusion");
        assert!(sessions.consume(&wrong, Role::Front, now).is_none(), "a failed presentation burns the ticket");
        let duel = sessions.ticket(&login, Role::Duel, now).unwrap();
        assert_eq!(sessions.consume(&duel, Role::Duel, now).unwrap().0, identity);
    }

    #[test]
    fn expired_and_unknown_credentials_cannot_open_a_connection() {
        let mut sessions = Sessions::default();
        let now = Instant::now();
        assert!(sessions.ticket(&[0; 32], Role::Front, now).is_none());
        assert!(sessions.consume(&[0; 32], Role::Front, now).is_none());
        let login = sessions.login(1001, now).unwrap();
        let ticket = sessions.ticket(&login, Role::Front, now).unwrap();
        assert!(sessions.consume(&ticket, Role::Front, now + TICKET_TTL).is_none());
        assert!(sessions.ticket(&login, Role::Duel, now + SESSION_TTL).is_none());
    }

    #[test]
    fn the_newest_login_of_an_account_ends_its_older_session() {
        let mut sessions = Sessions::default();
        let now = Instant::now();
        let first = sessions.login(1001, now).unwrap();
        let other = sessions.login(1002, now).unwrap();
        let lease = sessions.lease(sessions.sessions[&first].identity).unwrap();
        let second = sessions.login(1001, now).unwrap();
        assert!(sessions.ticket(&first, Role::Front, now).is_none(), "the older session of 1001 is gone");
        assert!(lease.has_changed().is_err(), "its connections see it end");
        assert!(sessions.ticket(&second, Role::Front, now).is_some());
        assert!(sessions.ticket(&other, Role::Front, now).is_some(), "another account is untouched");
    }

    #[test]
    fn logout_revokes_the_session_and_its_unspent_tickets() {
        let mut sessions = Sessions::default(); let now = Instant::now();
        let root = sessions.login(1001, now).unwrap();
        let ticket = sessions.ticket(&root, Role::Front, now).unwrap();
        sessions.logout(&root);
        assert!(sessions.ticket(&root, Role::Duel, now).is_none());
        assert!(sessions.consume(&ticket, Role::Front, now).is_none());
        sessions.logout(&root); // retry is harmless
        assert!(sessions.login(1001, now).is_some());
    }
}
