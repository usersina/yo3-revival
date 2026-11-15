//! The launcher's database on a thread of its own: the window never waits for a disk write, and writes happen in the
//! order they were asked for (two quick banlist changes can't land the wrong way round).
use std::path::Path;
use std::sync::mpsc;
use crate::{LauncherDb, Settings};

type Job = Box<dyn FnOnce(&mut LauncherDb) + Send>;

/// A handle to the database thread. Cloning it is cheap; the thread ends when the last handle is dropped.
#[derive(Clone)]
pub struct Store { jobs: mpsc::Sender<Job> }

impl Store {
    /// Opens `launcher.db` under `root` and returns its settings as they are now.
    pub fn open(root: &Path) -> anyhow::Result<(Self, Settings)> {
        let mut db = LauncherDb::open(root)?;
        let settings = db.settings()?;
        let (jobs, queue) = mpsc::channel::<Job>();
        std::thread::Builder::new().name("launcher-db".into()).spawn(move || { for job in queue { job(&mut db) } })?;
        Ok((Self { jobs }, settings))
    }

    /// Runs `work` on the database thread, after every job asked for before it.
    pub fn run<T: Send + 'static>(&self, work: impl FnOnce(&mut LauncherDb) -> anyhow::Result<T> + Send + 'static)
        -> impl Future<Output = anyhow::Result<T>> + Send + 'static {
        let (reply, answer) = tokio::sync::oneshot::channel();
        let sent = self.jobs.send(Box::new(move |db| { let _ = reply.send(work(db)); }));
        async move {
            sent.map_err(|_| anyhow::anyhow!("The launcher's database closed"))?;
            answer.await.map_err(|_| anyhow::anyhow!("The launcher's database stopped"))?
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> std::path::PathBuf {
        let at = std::env::temp_dir().join(format!("yo3-store-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&at).unwrap();
        at
    }

    #[tokio::test]
    async fn writes_run_off_the_caller_thread_in_order() {
        let root = scratch();
        let (store, settings) = Store::open(root.as_path()).unwrap();
        assert_eq!(settings.banlist, "");
        let caller = std::thread::current().id();
        let first = store.run(move |db| { assert_ne!(std::thread::current().id(), caller); db.save_banlist("4") });
        let second = store.run(|db| db.save_banlist("7"));
        assert_eq!(first.await.unwrap(), "4");
        assert_eq!(second.await.unwrap(), "7");
        assert_eq!(store.run(|db| db.settings()).await.unwrap().banlist, "7", "the later write wins");
    }
}
