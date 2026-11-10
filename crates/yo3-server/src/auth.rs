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
    let cert = rcgen::generate_simple_self_signed(names)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut key = options.open(&private_key)?;
    key.write_all(cert.signing_key.serialize_pem().as_bytes())?;
    let result = std::fs::OpenOptions::new().write(true).create_new(true).open(&certificate)
        .and_then(|mut f| f.write_all(cert.cert.pem().as_bytes()));
    if let Err(e) = result { let _ = std::fs::remove_file(&private_key); return Err(e.into()) }
    Ok(TlsConfig { port: 7443, certificate, private_key })
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
