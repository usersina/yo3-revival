//! Resolve the same Documents folder as the native client, and refuse resets while any client is open.
use std::path::PathBuf;
use anyhow::{Context, ensure};
use crate::{Backend, Settings};

fn is_game_name(name: &[u8]) -> bool { name.strip_suffix(b"\n").unwrap_or(name).eq_ignore_ascii_case(b"yo3.exe") }

#[cfg(windows)]
pub(super) fn data_directory(_backend: &Backend, _settings: &Settings) -> anyhow::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Shell::{SHGetFolderPathW, CSIDL_PERSONAL, SHGFP_TYPE_CURRENT};
    let mut folder=[0u16;260];
    let status=unsafe { SHGetFolderPathW(std::ptr::null_mut(),CSIDL_PERSONAL as i32,std::ptr::null_mut(),SHGFP_TYPE_CURRENT as u32,folder.as_mut_ptr()) };
    ensure!(status==0,"Cannot find the game's Documents folder (Windows status 0x{:08x}). Check that your Documents location is available, then retry.",status as u32);
    let end=folder.iter().position(|c|*c==0).context("Windows returned an incomplete Documents path")?;
    ensure!(end>0,"Windows returned an empty Documents folder");
    Ok(PathBuf::from(std::ffi::OsString::from_wide(&folder[..end])).join("Yu-Gi-Oh! ONLINE 3"))
}

#[cfg(not(windows))]
fn read_path(bytes: &[u8]) -> anyhow::Result<String> {
    let text=std::str::from_utf8(bytes).context("The game-data path helper returned invalid UTF-8")?;
    let path=text.strip_suffix('\n').unwrap_or(text);
    let path=path.strip_suffix('\r').unwrap_or(path);
    ensure!(!path.is_empty() && !path.contains(['\0','\r','\n']),"The game-data path helper returned an empty or unexpected path. Restore the matching application bundle, then retry.");
    Ok(path.to_owned())
}

#[cfg(not(windows))]
fn wine_command(settings: &Settings, name: &str) -> tokio::process::Command {
    let mut command=tokio::process::Command::new(name);
    if !settings.wine_prefix.is_empty() {command.env("WINEPREFIX",&settings.wine_prefix);}
    command.env("WINEDEBUG","-all");command
}

#[cfg(not(windows))]
async fn bounded_output(command: &mut tokio::process::Command, limit: std::time::Duration) -> anyhow::Result<Vec<u8>> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    let mut child=command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn()
        .context("Cannot start the game-data path helper. Check Wine and restore the complete application bundle")?;
    let stdout=child.stdout.take().context("The game-data path helper has no output pipe")?;
    let stderr=child.stderr.take().context("The game-data path helper has no diagnostic pipe")?;
    let read=async {
        let output=async move {let mut data=Vec::new();stdout.take(16385).read_to_end(&mut data).await?;ensure!(data.len()<=16384,"The game-data path helper returned too much output");Ok::<_,anyhow::Error>(data)};
        let errors=async move {let mut data=Vec::new();stderr.take(8193).read_to_end(&mut data).await?;ensure!(data.len()<=8192,"The game-data path helper returned too many diagnostic messages");Ok::<_,anyhow::Error>(data)};
        let (stdout,stderr)=tokio::try_join!(output,errors)?;
        let status=child.wait().await?;
        ensure!(status.success(),"Could not find the game's data folder ({status}). {}",String::from_utf8_lossy(&stderr).trim());
        Ok::<_,anyhow::Error>(stdout)
    };
    let result=tokio::time::timeout(limit,read).await;
    match result {
        Ok(Ok(bytes))=>Ok(bytes),
        failure=>{
            let _=child.kill().await;
            match failure {Ok(Err(error))=>Err(error),_=>anyhow::bail!("Finding the game's data folder took too long. Close any stuck Wine setup dialogs, check the selected Wine prefix, then retry.")}
        }
    }
}

#[cfg(not(windows))]
pub(super) fn data_directory(backend: &Backend, settings: &Settings) -> anyhow::Result<PathBuf> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let bytes=bounded_output(wine_command(settings,"wine").arg(&backend.runtime.injector).arg("--game-data-dir"),std::time::Duration::from_secs(30)).await?;
        let windows=read_path(&bytes)?;
        let drive=windows.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) && windows.as_bytes().get(1)==Some(&b':')
            && windows.as_bytes().get(2).is_some_and(|c|matches!(c,b'\\'|b'/'));
        ensure!(drive || windows.starts_with("\\\\"),"Wine returned an unexpected Windows game-data path");
        let bytes=bounded_output(wine_command(settings,"winepath").arg("-u").arg(&windows),std::time::Duration::from_secs(20)).await?;
        let folder=PathBuf::from(read_path(&bytes)?);
        ensure!(folder.is_absolute() && folder.file_name().is_some_and(|n|n=="Yu-Gi-Oh! ONLINE 3"),"Wine returned an unexpected game-data directory; reset was cancelled");
        Ok(folder)
    })
}

#[cfg(windows)]
pub(super) fn ensure_game_closed(_settings: &Settings) -> anyhow::Result<()> {
    use windows_sys::Win32::{Foundation::{CloseHandle, GetLastError, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS}};
    let snapshot=unsafe {CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0)};
    ensure!(snapshot!=INVALID_HANDLE_VALUE,"Cannot check whether the game is open: {}",std::io::Error::last_os_error());
    let result=(|| {
        let mut entry:PROCESSENTRY32W=unsafe {std::mem::zeroed()};entry.dwSize=std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found=unsafe {Process32FirstW(snapshot,&mut entry)};
        while found!=0 {
            let end=entry.szExeFile.iter().position(|c|*c==0).unwrap_or(entry.szExeFile.len());
            let name=String::from_utf16_lossy(&entry.szExeFile[..end]);
            ensure!(!is_game_name(name.as_bytes()),"Close every Yu-Gi-Oh! ONLINE 3 game window before resetting game data, including games opened outside this launcher.");
            found=unsafe {Process32NextW(snapshot,&mut entry)};
        }
        ensure!(unsafe {GetLastError()}==ERROR_NO_MORE_FILES,"Cannot finish checking whether the game is open: {}",std::io::Error::last_os_error());
        Ok(())
    })();
    unsafe {CloseHandle(snapshot)};result
}

#[cfg(not(windows))]
pub(super) fn ensure_game_closed(_settings: &Settings) -> anyhow::Result<()> {
    // Wine prefixes may map Documents to the same host folder, so check all clients.
    for entry in std::fs::read_dir("/proc").context("Cannot check whether the game is open; game-data reset was cancelled")? {
        let entry=entry.context("Cannot finish checking running game processes")?;
        if !entry.file_name().as_encoded_bytes().iter().all(u8::is_ascii_digit) {continue;}
        match std::fs::read(entry.path().join("comm")) {
            Ok(name)=>ensure!(!is_game_name(&name),"Close every Yu-Gi-Oh! ONLINE 3 game window before resetting game data, including games in other Wine prefixes."),
            Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},
            Err(error)=>return Err(error).context("Cannot inspect a running process; game-data reset was cancelled"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_process_matching_is_exact_and_case_insensitive() {
        for name in [b"yo3.exe".as_slice(),b"YO3.EXE\n",b"Yo3.Exe"] {assert!(is_game_name(name));}
        for name in [b"yo3-auth-launch.exe".as_slice(),b"yo3.exe.old",b"other-yo3.exe",b"yo3.exe extra",b""] {assert!(!is_game_name(name));}
    }
    #[cfg(not(windows))]
    #[test]
    fn path_protocol_preserves_unicode_and_rejects_extra_lines_or_missing_output() {
        let path="C:\\Users\\Test\\OneDrive\\文書\\Yu-Gi-Oh! ONLINE 3";
        assert_eq!(read_path(format!("{path}\r\n").as_bytes()).unwrap(),path);
        assert_eq!(read_path(path.as_bytes()).unwrap(),path);
        for bytes in [b"".as_slice(),b"\r\n",b"notice\nC:\\wrong",b"C:\\wrong\0",&[0xff]] {assert!(read_path(bytes).is_err());}
    }
    #[cfg(not(windows))]
    #[tokio::test]
    async fn path_helper_stalls_and_excess_diagnostics_are_bounded() {
        let started=std::time::Instant::now();
        let error=bounded_output(tokio::process::Command::new("sh").args(["-c","exec sleep 30"]),std::time::Duration::from_millis(50)).await.unwrap_err();
        assert!(error.to_string().contains("too long"));assert!(started.elapsed()<std::time::Duration::from_secs(2));
        let error=bounded_output(tokio::process::Command::new("sh").args(["-c","printf '%09000d' 0 >&2"]),std::time::Duration::from_secs(2)).await.unwrap_err();
        assert!(error.to_string().contains("too many diagnostic messages"));
    }
}
