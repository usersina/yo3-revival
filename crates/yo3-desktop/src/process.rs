//! Capture failures that happen before a child can open its own diagnostic session.
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use anyhow::Context;
use yo3_tunnel::diagnostics::{Session, redact_secrets};

pub(crate) struct Errors { tail: Arc<Mutex<String>>, done: mpsc::Receiver<()> }
impl Errors {
    pub fn finish(self) -> String {
        // An exited child normally closes stderr immediately. A descendant must not freeze the UI by keeping it open.
        let _=self.done.recv_timeout(std::time::Duration::from_millis(200));
        self.tail.lock().unwrap().clone()
    }
}
pub(crate) fn spawn(cmd: &mut Command, session: &Session, label: &'static str) -> anyhow::Result<(Child, Errors)> {
    cmd.stderr(Stdio::piped());
    let mut child=cmd.spawn().with_context(|| format!("Could not open {}. Check that the complete application is extracted and the file is allowed to run",cmd.get_program().to_string_lossy()))?;
    let mut stream=child.stderr.take().unwrap(); let log=session.clone();
    let tail=Arc::new(Mutex::new(String::new())); let output=tail.clone(); let (send,done)=mpsc::channel();
    let reader=std::thread::Builder::new().name("child-errors".into()).spawn(move || {
        let mut writer=log.writer(); let mut line=Vec::new(); let mut overflow=false; let mut private=false;
        let mut emit=|bytes: &[u8],overflow: bool| {
            let text=String::from_utf8_lossy(bytes);
            if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") { private=true; }
            let clean=if overflow { "[overlong diagnostic line omitted]".into() } else if private { "[private key omitted]".into() } else { redact_secrets(&text,&[]) };
            if text.contains("-----END") && text.contains("PRIVATE KEY-----") { private=false; }
            let _=writeln!(writer,"{label}: {clean}");
            let mut tail=output.lock().unwrap();
            if clean.len()<=4096 { tail.push_str(&clean); tail.push('\n'); }
            while tail.len()>8192 { if let Some(end)=tail.find('\n') { tail.drain(..=end); } else { tail.clear(); } }
        };
        let mut buffer=[0;4096];
        loop {
            let n=match stream.read(&mut buffer) { Ok(0)|Err(_)=>break,Ok(n)=>n };
            for byte in &buffer[..n] {
                if *byte==b'\n' { emit(&line,overflow); line.clear(); overflow=false; }
                else if line.len()<16384 { line.push(*byte); } else { overflow=true; }
            }
        }
        if !line.is_empty() || overflow { emit(&line,overflow); }
        drop(writer); let _=send.send(());
    });
    if let Err(error)=reader { let _=child.kill(); let _=child.wait(); return Err(error.into()) }
    Ok((child,Errors{tail,done}))
}
