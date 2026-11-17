//! Filesystem-only staff transport for the separate nginx container. No host management socket is shared.
use std::{fs,io,path::{Path,PathBuf},os::unix::fs::{FileTypeExt,MetadataExt,OpenOptionsExt,PermissionsExt}};
use tokio::net::UnixListener;

pub struct Bound { listener:UnixListener, path:PathBuf, inode:u64, _lock:fs::File }
impl Bound {
    pub fn bind(path:&Path)->io::Result<Self>{
        let lock_path=path.with_extension("socket-lock");
        if fs::symlink_metadata(&lock_path).is_ok_and(|m|!m.is_file()||m.file_type().is_symlink()){
            return Err(io::Error::other("Dashboard socket lock must be a regular file"))
        }
        let lock=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).open(lock_path)?;
        lock.try_lock().map_err(|e|io::Error::other(format!("Dashboard socket is in use: {e}")))?;
        match fs::symlink_metadata(path){
            Ok(m)=>{
                if !m.file_type().is_socket(){return Err(io::Error::other("Dashboard socket path already exists and is not a socket"))}
                match std::os::unix::net::UnixStream::connect(path){
                    Ok(_)=>return Err(io::Error::new(io::ErrorKind::AddrInUse,"Dashboard socket is already accepting connections")),
                    Err(e) if e.kind()==io::ErrorKind::ConnectionRefused=>fs::remove_file(path)?,
                    Err(e)=>return Err(e),
                }
            }
            Err(e) if e.kind()==io::ErrorKind::NotFound=>{},Err(e)=>return Err(e),
        }
        let listener=UnixListener::bind(path)?;
        let mut bound=Self{listener,path:path.into(),inode:fs::symlink_metadata(path)?.ino(),_lock:lock};
        if let Err(e)=fs::set_permissions(path,fs::Permissions::from_mode(0o660)){bound.remove();return Err(e)}
        Ok(bound)
    }
    fn remove(&mut self){
        if fs::symlink_metadata(&self.path).is_ok_and(|m|m.file_type().is_socket()&&m.ino()==self.inode){let _=fs::remove_file(&self.path);}
    }
}
impl Drop for Bound{fn drop(&mut self){self.remove()}}
pub async fn serve(bound:Bound,state:crate::dashboard::Dashboard,origin:crate::dashboard::ProxyOrigin){
    let router=crate::dashboard::proxy_router(state,origin).layer(axum::Extension(axum::extract::ConnectInfo("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap())));
    tracing::info!("staff dashboard API socket {} (no host-token authority)",bound.path.display());
    loop{
        let (stream,_)=match bound.listener.accept().await{Ok(s)=>s,Err(e)=>{tracing::warn!("Dashboard socket accept failed: {e}");tokio::time::sleep(std::time::Duration::from_millis(100)).await;continue}};
        let service=router.clone();
        tokio::spawn(async move{
            let mut http=hyper::server::conn::http1::Builder::new();
            http.keep_alive(false).half_close(true).timer(hyper_util::rt::TokioTimer::new()).header_read_timeout(std::time::Duration::from_secs(10));
            if let Err(e)=http.serve_connection(hyper_util::rt::TokioIo::new(stream),hyper_util::service::TowerToHyperService::new(service)).await{tracing::debug!("Dashboard socket connection ended: {e}")}
        });
    }
}
