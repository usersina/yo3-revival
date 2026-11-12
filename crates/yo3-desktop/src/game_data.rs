//! Reversible resets of the stock client's Documents folder, independent of Revival's account storage.
use std::{fs,io,path::{Path,PathBuf}};
use anyhow::{Context,ensure};
use crate::{Backend,Settings,game_data_platform};

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Preview { pub directory: PathBuf, pub settings_present: bool, pub full_present: bool }
#[derive(Clone,Debug)]
pub struct ResetResult { pub backup: PathBuf }

impl Backend {
    pub fn preview_game_data(&self, settings: &Settings) -> anyhow::Result<Preview> {
        preview_directory(&game_data_platform::data_directory(self,settings)?,&self.game_data_protected(settings))
    }
    pub fn reset_game_data(&self, settings: &Settings, preview: &Preview, full: bool) -> anyhow::Result<ResetResult> {
        ensure!(!self.game_running(),"Close the game before resetting its local data.");
        let mut owned=self.game.lock().unwrap();
        ensure!(owned.as_mut().is_none_or(|child|child.try_wait().is_ok_and(|status|status.is_some())),"Close the game before resetting its local data.");
        game_data_platform::ensure_game_closed(settings)?;
        let directory=game_data_platform::data_directory(self,settings)?;
        let result=reset_directory_checked(&directory,&self.game_data_protected(settings),preview,full,&mut |from,to|fs::rename(from,to),&mut ||game_data_platform::ensure_game_closed(settings))?;
        self.diagnostics.line(&format!("Reset client {}. Original data backed up to {}",if full{"local data"}else{"display and login settings"},result.backup.display()));
        Ok(result)
    }
    fn game_data_protected(&self, settings: &Settings) -> Vec<PathBuf> {
        let mut paths=vec![self.root.clone(),PathBuf::from(&settings.game_dir),self.data_dir(settings),self.runtime.engine.clone()];
        for file in [&self.runtime.server,&self.runtime.launcher,&self.runtime.injector,&self.runtime.dll] {
            if let Some(parent)=file.parent() { paths.push(parent.to_path_buf()); }
        }
        paths
    }
}

fn preview_directory(directory: &Path, protected: &[PathBuf]) -> anyhow::Result<Preview> {
    ensure_plain_path(directory)?;
    let directory=resolved(directory)?;
    for path in protected {
        let path=resolved(path)?;
        ensure!(!overlaps(&directory,&path),"The client data folder overlaps protected application, game or server files: {}. Correct the folders before resetting.",path.display());
    }
    let Some(entry)=metadata(&directory)? else { return Ok(Preview{directory,settings_present:false,full_present:false}) };
    ensure!(entry.is_dir(),"The client data path is not a folder: {}.",directory.display());
    for name in ["server.db","server.key","launcher.db"] {
        ensure!(metadata(&directory.join(name))?.is_none(),"The selected client folder contains protected {name}; nothing was reset.");
    }
    let options=metadata(&directory.join("options.y3o"))?;let users=metadata(&directory.join("users"))?;
    let settings_present=options.is_some() || users.is_some();
    let full_present=fs::read_dir(&directory).with_context(||format!("Could not inspect client data folder {}",directory.display()))?.next().transpose()?.is_some();
    Ok(Preview{directory,settings_present,full_present})
}

#[cfg(test)]
fn reset_directory<F: FnMut(&Path,&Path)->io::Result<()>>(directory: &Path,protected: &[PathBuf],preview: &Preview,full: bool,rename: &mut F) -> anyhow::Result<ResetResult> {
    reset_directory_checked(directory,protected,preview,full,rename,&mut ||Ok(()))
}
fn reset_directory_checked<F: FnMut(&Path,&Path)->io::Result<()>,G: FnMut()->anyhow::Result<()>>(directory: &Path,protected: &[PathBuf],preview: &Preview,full: bool,rename: &mut F,before_mutation: &mut G) -> anyhow::Result<ResetResult> {
    let current=preview_directory(directory,protected)?;
    ensure!(&current==preview,"The client data folder changed after confirmation. Review the folder and confirm the reset again; nothing was reset.");
    ensure!(if full {current.full_present}else{current.settings_present},"There is no {} to reset. No files were changed.",if full{"local client data"}else{"display or login settings"});
    let mut names=Vec::new();
    for name in ["options.y3o","users"] {if metadata(&directory.join(name))?.is_some(){names.push(name);}}
    if full {ensure_movable_tree(directory)?;} else {for name in &names {ensure_movable_tree(&directory.join(name))?;}}
    let backup=create_backup(&current.directory,protected)?;
    before_mutation().context("No client data was changed; close every running game before trying again")?;
    if full {
        ensure_location(directory,protected,&current.directory)?;ensure_backup(&backup)?;
        ensure!(metadata(&backup.join("data"))?.is_none(),"The backup destination already exists; no client data was changed.");
        rename(directory,&backup.join("data")).with_context(||format!("Could not move the client data to backup {}. Close the game and check folder permissions before trying again; the original folder was not reset.",backup.display()))?;
        return Ok(ResetResult{backup})
    }
    let mut moved=Vec::new();
    for name in names {
        let attempt=(|| -> anyhow::Result<()> {
            ensure_location(directory,protected,&current.directory)?;ensure_backup(&backup)?;
            ensure_plain_path(&directory.join(name))?;
            ensure!(metadata(&backup.join(name))?.is_none(),"The backup destination already exists");
            rename(&directory.join(name),&backup.join(name)).with_context(||format!("Could not back up {name}"))?;
            Ok(())
        })();
        if let Err(error)=attempt {
            let mut failures=Vec::new();
            for name in moved.into_iter().rev() {
                let restored=(|| -> anyhow::Result<()> {
                    ensure_location(directory,protected,&current.directory)?;ensure_backup(&backup)?;
                    ensure_plain_path(&backup.join(name))?;
                    ensure!(metadata(&directory.join(name))?.is_none(),"A new {name} appeared; refusing to replace it");
                    rename(&backup.join(name),&directory.join(name)).with_context(||format!("Could not restore {name}"))?;
                    Ok(())
                })();
                if let Err(error)=restored {failures.push(format!("{name}: {error:#}"));}
            }
            if failures.is_empty() {anyhow::bail!("Reset failed: {error:#}. Original client data was restored. Close the game and check folder permissions before trying again. Backup folder: {}",backup.display())}
            anyhow::bail!("Reset failed: {error:#}. Some files could not be restored: {}. Close the game and restore manually from {}; the backup has been kept.",failures.join("; "),backup.display())
        }
        moved.push(name);
    }
    Ok(ResetResult{backup})
}
fn ensure_movable_tree(path: &Path) -> anyhow::Result<()> {
    let mut pending=vec![path.to_path_buf()];
    while let Some(path)=pending.pop() {
        ensure_plain_path(&path)?;
        let entry=metadata(&path)?.with_context(||format!("Client data changed while checking {}. Review the reset again.",path.display()))?;
        if entry.is_dir() {for entry in fs::read_dir(&path).with_context(||format!("Could not inspect {}",path.display()))? {pending.push(entry?.path());}}
    }
    Ok(())
}
fn ensure_location(directory: &Path,protected: &[PathBuf],expected: &Path) -> anyhow::Result<()> {
    ensure!(preview_directory(directory,protected)?.directory==expected,"The client data location changed while resetting; refusing to move files.");
    ensure!(metadata(directory)?.is_some_and(|entry|entry.is_dir()),"The original client data folder disappeared while resetting.");
    Ok(())
}
fn ensure_backup(backup: &Path) -> anyhow::Result<()> {
    ensure_plain_path(backup)?;
    ensure!(resolved(backup)?==backup && metadata(backup)?.is_some_and(|entry|entry.is_dir()),"The backup location changed; refusing to move files.");
    Ok(())
}
fn create_backup(directory: &Path,protected: &[PathBuf]) -> anyhow::Result<PathBuf> {
    use std::sync::atomic::{AtomicU64,Ordering};
    static NEXT: AtomicU64=AtomicU64::new(0);
    let root=directory.parent().context("The client data folder has no Documents parent")?.join(".yo3-revival-backups");
    ensure_plain_path(&root)?;
    for path in protected {ensure!(!overlaps(&resolved(&root)?,&resolved(path)?),"The backup folder overlaps protected application, game or server files; nothing was reset.");}
    yo3_tunnel::diagnostics::private_dir(&root).with_context(||format!("Could not create a private backup folder {}. Check folder permissions; nothing was reset.",root.display()))?;
    for _ in 0..100 {
        let backup=root.join(format!("{}-{}-{}",chrono::Utc::now().format("%Y-%m-%d_%H%M%S-%fZ"),std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        let builder=fs::DirBuilder::new();
        #[cfg(unix)] let builder={use std::os::unix::fs::DirBuilderExt;let mut builder=builder;builder.mode(0o700);builder};
        match builder.create(&backup) {Ok(())=>return Ok(backup),Err(error) if error.kind()==io::ErrorKind::AlreadyExists=>continue,Err(error)=>return Err(error).context("Could not create the client backup; nothing was reset")}
    }
    anyhow::bail!("Could not create a unique client backup; nothing was reset.")
}

fn metadata(path: &Path) -> anyhow::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) { Ok(value)=>Ok(Some(value)),Err(error) if error.kind()==io::ErrorKind::NotFound=>Ok(None),Err(error)=>Err(error).with_context(||format!("Could not inspect {}",path.display())) }
}
fn ensure_plain_path(path: &Path) -> anyhow::Result<()> {
    if let Some(metadata)=metadata(path)? {
        let redirected=metadata.file_type().is_symlink();
        #[cfg(windows)] let redirected={use std::os::windows::fs::MetadataExt;redirected || metadata.file_attributes()&0x400!=0};
        ensure!(!redirected,"Refusing to reset a symbolic link or redirected client data entry: {}.",path.display());
    }
    Ok(())
}
fn resolved(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute=if path.is_absolute() {path.to_path_buf()} else {std::env::current_dir()?.join(path)};
    let mut current=absolute.as_path();let mut missing=Vec::new();
    loop { match fs::canonicalize(current) {
        Ok(mut found)=>{for part in missing.into_iter().rev(){found.push(part)}return Ok(found)},
        Err(error) if error.kind()==io::ErrorKind::NotFound=>{
            missing.push(current.file_name().context("Cannot safely resolve this folder")?.to_os_string());
            current=current.parent().context("Cannot safely resolve this folder")?;
        }
        Err(error)=>return Err(error).with_context(||format!("Could not resolve {}",path.display())),
    } }
}
fn overlaps(a: &Path,b: &Path) -> bool {
    #[cfg(windows)] {
        let parts=|path: &Path|path.components().map(|part|part.as_os_str().to_string_lossy().to_lowercase()).collect::<Vec<_>>();
        let (a,b)=(parts(a),parts(b));a.starts_with(&b)||b.starts_with(&a)
    }
    #[cfg(not(windows))] {a.starts_with(b)||b.starts_with(a)}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64,Ordering};
    static NEXT: AtomicU64=AtomicU64::new(0);
    struct Fixture { root: PathBuf, data: PathBuf, protected: Vec<PathBuf> }
    impl Fixture {
        fn new() -> Self {
            let root=std::env::temp_dir().join(format!("yo3-game-data-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            let data=root.join("Documents/Yu-Gi-Oh! ONLINE 3");fs::create_dir_all(&data).unwrap();
            let protected=vec![root.join("revival"),root.join("installed-game"),root.join("server")];
            for directory in &protected {fs::create_dir_all(directory).unwrap();fs::write(directory.join("keep"),b"protected").unwrap();}
            Self{root,data,protected}
        }
        fn seed(&self) {
            for (name,bytes) in [("options.y3o","display settings"),("users/account.y3u","stock login"),("players/Duelist/Duelist.y3p","character preferences"),("players/Duelist/decks/custom.y3d","manual deck"),("players/Duelist/revival-starter.initialized","manual starter marker")] {
                let file=self.data.join(name);fs::create_dir_all(file.parent().unwrap()).unwrap();fs::write(file,bytes).unwrap();
            }
        }
        fn preview(&self) -> Preview {preview_directory(&self.data,&self.protected).unwrap()}
        fn reset(&self,full: bool) -> anyhow::Result<ResetResult> {reset_directory(&self.data,&self.protected,&self.preview(),full,&mut |from,to|fs::rename(from,to))}
        fn backups(&self) -> Vec<PathBuf> {fs::read_dir(self.data.parent().unwrap().join(".yo3-revival-backups")).map(|entries|entries.map(|entry|entry.unwrap().path()).collect()).unwrap_or_default()}
        fn assert_protected(&self) {for directory in &self.protected {assert_eq!(fs::read(directory.join("keep")).unwrap(),b"protected");}}
    }
    impl Drop for Fixture {fn drop(&mut self){let _=fs::remove_dir_all(&self.root);}}

    #[test]
    fn game_data_default_reset_keeps_all_player_files_and_backs_up_only_settings() {
        let fixture=Fixture::new();fixture.seed();let backup=fixture.reset(false).unwrap().backup;
        assert!(!fixture.data.join("options.y3o").exists());assert!(!fixture.data.join("users").exists());
        for (name,bytes) in [("Duelist.y3p","character preferences"),("decks/custom.y3d","manual deck"),("revival-starter.initialized","manual starter marker")] {
            assert_eq!(fs::read_to_string(fixture.data.join("players/Duelist").join(name)).unwrap(),bytes);
        }
        assert_eq!(fs::read_to_string(backup.join("options.y3o")).unwrap(),"display settings");
        assert_eq!(fs::read_to_string(backup.join("users/account.y3u")).unwrap(),"stock login");
        assert!(!backup.join("players").exists());fixture.assert_protected();
    }
    #[test]
    fn game_data_full_reset_moves_the_entire_original_tree_to_backup_data() {
        let fixture=Fixture::new();fixture.seed();let backup=fixture.reset(true).unwrap().backup;
        assert!(!fixture.data.exists());assert_eq!(fs::read_to_string(backup.join("data/players/Duelist/decks/custom.y3d")).unwrap(),"manual deck");
        assert_eq!(fs::read_to_string(backup.join("data/options.y3o")).unwrap(),"display settings");fixture.assert_protected();
    }
    #[test]
    fn game_data_failed_second_move_rolls_back_the_first_move() {
        let fixture=Fixture::new();fixture.seed();let mut moves=0;
        let result=reset_directory(&fixture.data,&fixture.protected,&fixture.preview(),false,&mut |from,to|{
            moves+=1;if moves==2 {Err(io::ErrorKind::PermissionDenied.into())} else {fs::rename(from,to)}
        });
        assert!(format!("{:#}",result.unwrap_err()).contains("restored"));
        assert_eq!(fs::read_to_string(fixture.data.join("options.y3o")).unwrap(),"display settings");
        assert_eq!(fs::read_to_string(fixture.data.join("users/account.y3u")).unwrap(),"stock login");fixture.assert_protected();
    }
    #[test]
    fn game_data_failed_rollback_keeps_the_backup_and_reports_its_path() {
        let fixture=Fixture::new();fixture.seed();let mut moves=0;
        let result=reset_directory(&fixture.data,&fixture.protected,&fixture.preview(),false,&mut |from,to|{
            moves+=1;if moves>=2 {Err(io::ErrorKind::PermissionDenied.into())} else {fs::rename(from,to)}
        });
        let error=format!("{:#}",result.unwrap_err());assert!(error.contains("restore manually"),"{error}");
        let backups=fixture.backups();assert_eq!(backups.len(),1);assert!(error.contains(backups[0].to_string_lossy().as_ref()));
        assert_eq!(fs::read_to_string(backups[0].join("options.y3o")).unwrap(),"display settings");
        assert_eq!(fs::read_to_string(fixture.data.join("users/account.y3u")).unwrap(),"stock login");
    }
    #[test]
    fn game_data_no_matching_scope_creates_no_backup_or_changes() {
        let fixture=Fixture::new();assert!(!fixture.preview().full_present);assert!(fixture.reset(true).is_err());assert!(fixture.backups().is_empty());
        fs::remove_dir(&fixture.data).unwrap();assert!(!fixture.preview().full_present);assert!(fixture.reset(true).is_err());assert!(fixture.backups().is_empty());
        fs::create_dir_all(fixture.data.join("players/Duelist/decks")).unwrap();
        assert!(fixture.preview().full_present);assert!(!fixture.preview().settings_present);assert!(fixture.reset(false).is_err());assert!(fixture.backups().is_empty());
    }
    #[test]
    fn game_data_protected_overlaps_and_server_markers_are_refused() {
        let fixture=Fixture::new();
        for path in [&fixture.data,fixture.data.parent().unwrap(),&fixture.data.join("nested-server")] {
            assert!(preview_directory(&fixture.data,&[path.to_path_buf()]).is_err());
        }
        for marker in ["server.db","server.key","launcher.db"] {
            fs::write(fixture.data.join(marker),b"do not touch").unwrap();assert!(preview_directory(&fixture.data,&fixture.protected).is_err());fs::remove_file(fixture.data.join(marker)).unwrap();
        }
        assert!(fixture.backups().is_empty());fixture.assert_protected();
    }
    #[test]
    fn game_data_changed_confirmation_is_refused_before_creating_a_backup() {
        let fixture=Fixture::new();fixture.seed();let mut preview=fixture.preview();preview.directory=fixture.root.join("somewhere-else");
        assert!(reset_directory(&fixture.data,&fixture.protected,&preview,false,&mut |from,to|fs::rename(from,to)).is_err());assert!(fixture.backups().is_empty());
    }
    #[test]
    fn game_data_game_guard_is_refreshed_after_backup_setup_and_before_any_move() {
        let fixture=Fixture::new();fixture.seed();let mut moved=false;
        let error=reset_directory_checked(&fixture.data,&fixture.protected,&fixture.preview(),false,&mut |from,to|{moved=true;fs::rename(from,to)},&mut ||{
            assert_eq!(fixture.backups().len(),1,"resolve, scan and prepare backup before the final game check");
            anyhow::bail!("Another game opened")
        }).unwrap_err();
        assert!(format!("{error:#}").contains("Another game opened"));assert!(!moved);
        assert_eq!(fs::read_to_string(fixture.data.join("options.y3o")).unwrap(),"display settings");fixture.assert_protected();
    }
    #[test]
    fn game_data_rollback_never_replaces_newly_created_settings() {
        let fixture=Fixture::new();fixture.seed();let mut moves=0;
        let error=reset_directory(&fixture.data,&fixture.protected,&fixture.preview(),false,&mut |from,to|{
            moves+=1;if moves==2 {fs::write(fixture.data.join("options.y3o"),b"new settings")?;Err(io::ErrorKind::PermissionDenied.into())} else {fs::rename(from,to)}
        }).unwrap_err();
        assert!(format!("{error:#}").contains("restore manually"));assert_eq!(fs::read_to_string(fixture.data.join("options.y3o")).unwrap(),"new settings");
        assert_eq!(fs::read_to_string(fixture.backups()[0].join("options.y3o")).unwrap(),"display settings");fixture.assert_protected();
    }
    #[test]
    fn game_data_full_backup_never_replaces_an_existing_destination() {
        let fixture=Fixture::new();fixture.seed();
        let error=reset_directory_checked(&fixture.data,&fixture.protected,&fixture.preview(),true,&mut |from,to|fs::rename(from,to),&mut ||{
            fs::create_dir(fixture.backups()[0].join("data"))?;Ok(())
        }).unwrap_err();
        assert!(format!("{error:#}").contains("destination already exists"));assert_eq!(fs::read_to_string(fixture.data.join("options.y3o")).unwrap(),"display settings");fixture.assert_protected();
    }
    #[test]
    fn game_data_backup_names_are_unique_and_private() {
        let fixture=Fixture::new();fixture.seed();let first=fixture.reset(false).unwrap().backup;
        fs::write(fixture.data.join("options.y3o"),b"new display settings").unwrap();let second=fixture.reset(false).unwrap().backup;
        assert_ne!(first,second);assert_eq!(fs::read_to_string(first.join("options.y3o")).unwrap(),"display settings");
        assert_eq!(fs::read_to_string(second.join("options.y3o")).unwrap(),"new display settings");
        #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;for folder in [&first,&second,first.parent().unwrap()] {assert_eq!(fs::metadata(folder).unwrap().permissions().mode()&0o077,0);}}
    }
    #[test]
    #[cfg(windows)]
    fn game_data_windows_drive_and_unc_roots_are_protected_ancestors() {
        assert!(overlaps(Path::new(r"\\?\C:\Documents\Yu-Gi-Oh! ONLINE 3"),Path::new(r"\\?\C:\")));
        assert!(overlaps(Path::new(r"\\?\UNC\host\share\Documents\Yu-Gi-Oh! ONLINE 3"),Path::new(r"\\?\UNC\host\share\")));
        assert!(!overlaps(Path::new(r"\\?\C:\Documents\game"),Path::new(r"\\?\C:\Documents\games")));
    }
    #[test]
    #[cfg(unix)]
    fn game_data_documents_parent_redirect_is_allowed_but_backup_leaf_redirect_is_not() {
        use std::os::unix::fs::symlink;
        let fixture=Fixture::new();fixture.seed();
        let documents=fixture.root.join("Documents");let redirected=fixture.root.join("real-documents");fs::rename(&documents,&redirected).unwrap();symlink(&redirected,&documents).unwrap();
        let backup=fixture.reset(false).unwrap().backup;assert!(backup.starts_with(&redirected));
        fs::write(fixture.data.join("options.y3o"),b"new settings").unwrap();
        let backup_root=redirected.join(".yo3-revival-backups");fs::rename(&backup_root,redirected.join("previous-backups")).unwrap();
        let outside=fixture.root.join("outside");fs::create_dir(&outside).unwrap();symlink(&outside,&backup_root).unwrap();
        assert!(fixture.reset(false).is_err());assert_eq!(fs::read(fixture.data.join("options.y3o")).unwrap(),b"new settings");assert!(fs::read_dir(&outside).unwrap().next().is_none());
    }
    #[test]
    #[cfg(unix)]
    fn game_data_leaf_links_are_refused_and_default_reset_does_not_follow_players_links() {
        use std::os::unix::fs::symlink;
        let fixture=Fixture::new();let outside=fixture.root.join("outside");fs::create_dir_all(&outside).unwrap();fs::write(outside.join("keep"),b"untouched").unwrap();
        fs::remove_dir(&fixture.data).unwrap();symlink(&outside,&fixture.data).unwrap();assert!(preview_directory(&fixture.data,&fixture.protected).is_err());fs::remove_file(&fixture.data).unwrap();fs::create_dir(&fixture.data).unwrap();
        symlink(&outside,fixture.data.join("users")).unwrap();assert!(fixture.reset(false).is_err());assert!(fixture.backups().is_empty());fs::remove_file(fixture.data.join("users")).unwrap();
        fixture.seed();symlink(&outside,fixture.data.join("players/linked")).unwrap();
        assert!(fixture.reset(true).is_err());assert!(fixture.backups().is_empty());
        fixture.reset(false).unwrap();assert!(fs::symlink_metadata(fixture.data.join("players/linked")).unwrap().file_type().is_symlink());assert_eq!(fs::read(outside.join("keep")).unwrap(),b"untouched");
    }
}
