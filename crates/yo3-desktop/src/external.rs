//! Opening web pages and folders with the system's own application.

pub fn open(path: &str) -> anyhow::Result<()> {
    let result:anyhow::Result<()>= {
        #[cfg(windows)] {
            use std::os::windows::ffi::OsStrExt;
            let wide:Vec<u16>=std::ffi::OsStr::new(path).encode_wide().chain(Some(0)).collect();
            let result=unsafe {windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(),std::ptr::null(),wide.as_ptr(),std::ptr::null(),std::ptr::null(),windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL)};
            if result as isize>32 {Ok(())}else{Err(anyhow::anyhow!("No application could open this location (Windows error {})",result as isize))}
        }
        #[cfg(not(windows))] {wait_for_opener(std::process::Command::new("xdg-open").arg(path),std::time::Duration::from_secs(15))}
    };
    result.map_err(|e|anyhow::anyhow!("Could not open the browser or file manager. Copy this location and open it manually:\n{path}\n{e:#}"))
}
#[cfg(not(windows))]
fn wait_for_opener(cmd:&mut std::process::Command,limit:std::time::Duration)->anyhow::Result<()> {
    let mut child=cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?;
    let deadline=std::time::Instant::now()+limit;
    loop {
        if let Some(status)=child.try_wait()? {anyhow::ensure!(status.success(),"The system opener failed ({status})");return Ok(())}
        if std::time::Instant::now()>=deadline {let _=child.kill();let _=child.wait();anyhow::bail!("The system opener did not confirm within {} seconds",limit.as_secs());}
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

