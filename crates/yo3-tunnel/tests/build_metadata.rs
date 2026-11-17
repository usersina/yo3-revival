//! Cargo watch paths must exist in checkouts, linked worktrees and source archives.
use std::{path::{Path,PathBuf},process::{Command,Output}};
struct Fixture{root:PathBuf,program:PathBuf}
impl Fixture{
 fn new(name:&str)->Self{
  let root=std::env::temp_dir().join(format!("yo3-build-metadata-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();
  let program=root.join(format!("metadata{}",std::env::consts::EXE_SUFFIX));
  let output=Command::new("rustc").args(["--edition=2024",concat!(env!("CARGO_MANIFEST_DIR"),"/build.rs"),"-o"]).arg(&program).output().unwrap();
  assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));Self{root,program}
 }
 fn crate_dir(&self,checkout:&Path)->PathBuf{let path=checkout.join("crates/yo3-tunnel");std::fs::create_dir_all(&path).unwrap();std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"),"/build.rs"),path.join("build.rs")).unwrap();path}
 fn run(&self,crate_dir:&Path)->String{
  let out=Command::new(&self.program).current_dir(crate_dir).env("CARGO_MANIFEST_DIR",crate_dir).env("CARGO_PKG_VERSION","9.8.7").env_remove("YO3_BUILD_VERSION").env_remove("YO3_BUILD_COMMIT").output().unwrap();
  assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));String::from_utf8(out.stdout).unwrap()
 }
}
impl Drop for Fixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.root);}}
fn git(dir:&Path,args:&[&str])->Output{
 let hooks=dir.join(".empty-hooks");std::fs::create_dir_all(&hooks).unwrap();
 let out=Command::new("git").current_dir(dir).args(["-c","commit.gpgsign=false","-c"]).arg(format!("core.hooksPath={}",hooks.display())).args(["-c","user.name=Build Test","-c","user.email=build@example.invalid"]).args(args).output().unwrap();assert!(out.status.success(),"{args:?}: {}",String::from_utf8_lossy(&out.stderr));out
}
fn watches(output:&str,dir:&Path)->Vec<PathBuf>{output.lines().filter_map(|line|line.strip_prefix("cargo:rerun-if-changed=")).map(|path|{let path=dir.join(path);path.canonicalize().unwrap_or(path)}).collect()}
fn normalized(path:&Path)->PathBuf {
 if let Ok(path)=path.canonicalize(){return path}
 if let Some(parent)=path.ancestors().find(|p|p.exists()) && let Ok(canonical)=parent.canonicalize(){return canonical.join(path.strip_prefix(parent).unwrap())}
 path.to_path_buf()
}
fn assert_covers(watches:&[PathBuf],changed:&Path){let changed=normalized(changed);assert!(watches.iter().any(|p|p==&changed || p.is_dir() && changed.starts_with(p)),"the Git change at {} is not watched: {watches:?}",changed.display());}
#[test]
fn ordinary_linked_packed_and_detached_checkouts_watch_real_git_files(){
 let f=Fixture::new("git");let repo=f.root.join("repo");std::fs::create_dir(&repo).unwrap();let _=f.crate_dir(&repo);
 git(&repo,&["init","-b","main"]);git(&repo,&["add","."]);git(&repo,&["commit","-m","seed"]);
 let worktree=f.root.join("worktree");git(&repo,&["worktree","add","-b","feature",worktree.to_str().unwrap()]);
 for checkout in [&repo,&worktree]{
  let dir=checkout.join("crates/yo3-tunnel");let output=f.run(&dir);let paths=watches(&output,&dir);
  assert!(!paths.is_empty());assert!(paths.iter().all(|p|p.exists()),"Cargo will rebuild forever when a watched file is absent: {paths:?}");
  let actual=git(&dir,&["rev-parse","--path-format=absolute","--git-path","HEAD"]);let head=PathBuf::from(String::from_utf8(actual.stdout).unwrap().trim());assert_covers(&paths,&head);
  let reference=if checkout==&repo{"refs/heads/main"}else{"refs/heads/feature"};
  let actual=git(&dir,&["rev-parse","--path-format=absolute","--git-path",reference]);let reference_path=PathBuf::from(String::from_utf8(actual.stdout).unwrap().trim());assert_covers(&paths,&reference_path);
  let commit=String::from_utf8(git(&dir,&["rev-parse","--short=12","HEAD"]).stdout).unwrap();assert!(output.contains(&format!("cargo:rustc-env=YO3_COMMIT={}",commit.trim())));
 }
 git(&repo,&["pack-refs","--all","--prune"]);
 let dir=worktree.join("crates/yo3-tunnel");let paths=watches(&f.run(&dir),&dir);assert!(paths.iter().all(|p|p.exists()));
 let actual=git(&dir,&["rev-parse","--path-format=absolute","--git-path","refs/heads/feature"]);let reference_path=PathBuf::from(String::from_utf8(actual.stdout).unwrap().trim());assert!(!reference_path.exists());assert_covers(&paths,&reference_path);
 let packed=git(&dir,&["rev-parse","--path-format=absolute","--git-path","packed-refs"]);assert_covers(&paths,&PathBuf::from(String::from_utf8(packed.stdout).unwrap().trim()));
 git(&worktree,&["checkout","--detach"]);let paths=watches(&f.run(&dir),&dir);assert!(paths.iter().all(|p|p.exists()));
}
#[test]
fn an_archive_without_git_never_emits_a_missing_git_watch_path(){
 let f=Fixture::new("archive");let dir=f.crate_dir(&f.root.join("archive"));let output=f.run(&dir);let paths=watches(&output,&dir);
 assert!(paths.iter().all(|p|p.exists()),"source bundles must not force a rebuild on every command: {paths:?}");
 assert!(output.contains("cargo:rustc-env=YO3_COMMIT=unknown"));assert!(output.contains("cargo:rustc-env=YO3_VERSION=9.8.7-dev"));
}
