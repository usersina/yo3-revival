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

fn certificate_parameters(names: Vec<String>) -> anyhow::Result<rcgen::CertificateParams> {
    let mut params = rcgen::CertificateParams::new(names)?;
    params.not_before = std::time::SystemTime::now().into();
    params.not_before -= std::time::Duration::from_secs(300);
    params.not_after = params.not_before + std::time::Duration::from_secs(365 * 86400 + 300);
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

    pub fn lease(&self, identity: Identity) -> Option<tokio::sync::watch::Receiver<bool>> {
        self.sessions.values().find(|s| s.identity == identity).map(|s| s.revoked.subscribe())
    }
    fn prune(&mut self, now: Instant) {
        self.sessions.retain(|_, s| now < s.expires);
        self.tickets.retain(|_, t| now < t.expires && self.sessions.contains_key(&t.session));
    }

    pub fn login(&mut self, user_id: u32, now: Instant) -> Option<Token> {
        self.prune(now);
        if self.sessions.len() >= 1024 { return None }
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
    fn separate_logins_remain_separate_even_for_one_account() {
        let mut sessions = Sessions::default();
        let now = Instant::now();
        let ids: Vec<_> = [1001, 1002, 1001].into_iter().map(|user| {
            let login = sessions.login(user, now).unwrap();
            let ticket = sessions.ticket(&login, Role::Front, now).unwrap();
            sessions.consume(&ticket, Role::Front, now).unwrap().0
        }).collect();
        assert_eq!(ids.iter().map(|i| i.user_id).collect::<Vec<_>>(), [1001, 1002, 1001]);
        assert_ne!(ids[0].session_id, ids[1].session_id);
        assert_ne!(ids[0].session_id, ids[2].session_id);
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
