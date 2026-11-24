//! Small, fail-soft operating-system details for a user-reviewed bug report.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemInfo { pub os: String, pub arch: String, pub wine: Option<String>,
    /// The PC's specs for a report: the processor's name, its threads, the memory in MiB and the graphics card's name.
    #[serde(default)] pub cpu: Option<String>, #[serde(default)] pub threads: usize, #[serde(default)] pub memory_mb: Option<u64>, #[serde(default)] pub gpu: Option<String> }
impl SystemInfo {
    pub fn collect() -> Self {
        #[cfg(windows)]
        let os=windows_description(registry_string("ProductName").as_deref(),registry_string("EditionID").as_deref(),registry_string("DisplayVersion").as_deref(),
            registry_string("CurrentBuildNumber").and_then(|build|build.parse().ok()),registry_u32("UBR"));
        #[cfg(target_os="linux")]
        let os={
            use std::io::Read;
            let mut release=String::new();
            std::fs::File::open("/etc/os-release").ok().and_then(|file|file.take(65536).read_to_string(&mut release).ok()).and_then(|_|pretty_name(&release)).unwrap_or_else(||"Linux".into())
        };
        #[cfg(not(any(windows,target_os="linux")))]
        let os=std::env::consts::OS.into();
        #[cfg(target_os="linux")]
        let wine=std::thread::Builder::new().name("report-system-info".into()).spawn(|| {
            tokio::runtime::Builder::new_current_thread().enable_all().build().ok()?.block_on(
                version_output(tokio::process::Command::new("wine").arg("--version"),std::time::Duration::from_millis(1500)))
        }).ok().and_then(|thread|thread.join().ok()).flatten();
        #[cfg(not(target_os="linux"))]
        let wine=None;
        let threads=std::thread::available_parallelism().map_or(0,|n|n.get());
        #[cfg(windows)]
        let (cpu,memory_mb,gpu)=(registry_string_at("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0","ProcessorNameString").map(|s|s.trim().to_owned()),
            windows_memory_mb(),registry_string_at("SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0000","DriverDesc"));
        #[cfg(target_os="linux")]
        let (cpu,memory_mb,gpu)={
            let read=|path:&str|std::fs::read_to_string(path).unwrap_or_default();
            (cpu_model(&read("/proc/cpuinfo")),mem_total_mb(&read("/proc/meminfo")),linux_gpu())
        };
        #[cfg(not(any(windows,target_os="linux")))]
        let (cpu,memory_mb,gpu)=(None,None,None);
        Self {os,arch:std::env::consts::ARCH.into(),wine,cpu,threads,memory_mb,gpu}
    }
}
/// The processor's name in `/proc/cpuinfo` (its first `model name`).
#[cfg(any(target_os="linux",test))]
fn cpu_model(cpuinfo: &str) -> Option<String> {
    let value=cpuinfo.lines().find_map(|l|l.strip_prefix("model name").and_then(|r|r.split_once(':')).map(|(_,v)|v.trim().to_owned()))?;
    (!value.is_empty() && value.len()<=200 && !value.chars().any(char::is_control)).then_some(value)
}
/// `MemTotal` in `/proc/meminfo`, in MiB.
#[cfg(any(target_os="linux",test))]
fn mem_total_mb(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|l|l.strip_prefix("MemTotal:")).and_then(|r|r.split_whitespace().next()?.parse::<u64>().ok()).map(|kb|kb/1024)
}
/// The first display controller's name from `lspci` (if it is installed), bounded like the Wine probe.
#[cfg(target_os="linux")]
fn linux_gpu() -> Option<String> {
    let out=std::thread::Builder::new().name("report-gpu".into()).spawn(|| {
        tokio::runtime::Builder::new_current_thread().enable_all().build().ok()?.block_on(
            version_output(tokio::process::Command::new("sh").args(["-c","lspci 2>/dev/null | grep -m1 -E 'VGA|3D|Display' | cut -d: -f3-"]),std::time::Duration::from_millis(1500)))
    }).ok().and_then(|thread|thread.join().ok()).flatten()?;
    let out=out.trim().to_owned();
    (!out.is_empty() && out.len()<=200).then_some(out)
}
#[cfg(windows)]
fn windows_memory_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx,MEMORYSTATUSEX};
    let mut status:MEMORYSTATUSEX=unsafe{std::mem::zeroed()};status.dwLength=std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    (unsafe{GlobalMemoryStatusEx(&mut status)}!=0).then(||status.ullTotalPhys/(1024*1024))
}
#[cfg(any(windows,test))]
fn windows_description(product: Option<&str>, edition: Option<&str>, display: Option<&str>, build: Option<u32>, revision: Option<u32>) -> String {
    let mut name=match product.filter(|product|!product.is_empty()) {
        Some(product) if build.is_some_and(|build|build>=22000) && product.starts_with("Windows 10")=>product.replacen("Windows 10","Windows 11",1),
        Some(product)=>product.into(),
        None=>format!("{}{}",if build.is_some_and(|build|build>=22000){"Windows 11"}else{"Windows"},edition.filter(|edition|!edition.is_empty()).map(|edition|format!(" {edition}")).unwrap_or_default()),
    };
    if let Some(display)=display.filter(|display|!display.is_empty()) {name.push(' ');name.push_str(display);}
    if let Some(build)=build {let revision=revision.map(|revision|format!(".{revision}")).unwrap_or_default();name.push_str(&format!(" (build {build}{revision})"));}
    name
}
#[cfg(windows)]
fn registry_string(name: &str) -> Option<String> { registry_string_at("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",name) }
#[cfg(windows)]
fn registry_string_at(key: &str, name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW,HKEY_LOCAL_MACHINE,RRF_RT_REG_SZ,RRF_SUBKEY_WOW6464KEY};
    let key:Vec<u16>=key.encode_utf16().chain(Some(0)).collect();let name:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();
    let mut buffer=[0u16;256];let mut bytes=std::mem::size_of_val(&buffer) as u32;
    let status=unsafe {RegGetValueW(HKEY_LOCAL_MACHINE,key.as_ptr(),name.as_ptr(),RRF_RT_REG_SZ|RRF_SUBKEY_WOW6464KEY,std::ptr::null_mut(),buffer.as_mut_ptr().cast(),&mut bytes)};
    if status!=0 || bytes as usize>std::mem::size_of_val(&buffer) {return None}
    let end=buffer.iter().position(|value|*value==0)?;let text=String::from_utf16(&buffer[..end]).ok()?;
    (!text.is_empty() && !text.chars().any(char::is_control)).then_some(text)
}
#[cfg(windows)]
fn registry_u32(name: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::{RegGetValueW,HKEY_LOCAL_MACHINE,RRF_RT_REG_DWORD,RRF_SUBKEY_WOW6464KEY};
    let key:Vec<u16>="SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion".encode_utf16().chain(Some(0)).collect();let name:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();
    let mut value=0u32;let mut bytes=4u32;
    let status=unsafe {RegGetValueW(HKEY_LOCAL_MACHINE,key.as_ptr(),name.as_ptr(),RRF_RT_REG_DWORD|RRF_SUBKEY_WOW6464KEY,std::ptr::null_mut(),(&mut value as *mut u32).cast(),&mut bytes)};
    (status==0 && bytes==4).then_some(value)
}
#[cfg(any(target_os="linux",test))]
fn pretty_name(release: &str) -> Option<String> {
    let value=release.lines().find_map(|line|line.strip_prefix("PRETTY_NAME="))?.trim();
    let value=if let Some(quote @ ('\''|'"'))=value.chars().next() {value.strip_prefix(quote)?.strip_suffix(quote)?} else {value};
    let value=value.replace("\\\"","\"").replace("\\$","$").replace("\\`","`").replace("\\\\","\\");
    (!value.is_empty() && value.len()<=1024 && !value.chars().any(char::is_control)).then_some(value)
}
#[cfg(any(target_os="linux",all(test,not(windows))))]
async fn version_output(command: &mut tokio::process::Command, limit: std::time::Duration) -> Option<String> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    let mut child=command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true).spawn().ok()?;
    let stdout=child.stdout.take()?;
    let result=tokio::time::timeout(limit,async {
        let mut bytes=Vec::new();stdout.take(1025).read_to_end(&mut bytes).await.ok()?;
        if bytes.len()>1024 {return None}
        let status=child.wait().await.ok()?;let version=String::from_utf8(bytes).ok()?.trim().to_owned();
        (status.success() && !version.is_empty() && !version.chars().any(char::is_control)).then_some(version)
    }).await.ok().flatten();
    if child.try_wait().ok().flatten().is_none() {let _=child.kill().await;let _=child.wait().await;}
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_build_distinguishes_eleven_even_when_registry_product_name_says_ten() {
        assert_eq!(windows_description(Some("Windows 10 Pro"),Some("Professional"),Some("24H2"),Some(26100),Some(1234)),"Windows 11 Pro 24H2 (build 26100.1234)");
        assert_eq!(windows_description(Some("Windows 10 Home"),None,Some("22H2"),Some(19045),None),"Windows 10 Home 22H2 (build 19045)");
        assert_eq!(windows_description(Some("Windows Server 2025 Standard"),None,None,Some(26100),None),"Windows Server 2025 Standard (build 26100)");
        assert_eq!(windows_description(None,None,None,None,None),"Windows");
    }
    #[test]
    fn the_pcs_processor_and_memory_are_read_from_proc() {
        assert_eq!(cpu_model("processor\t: 0\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\nflags: x\n"),Some("AMD Ryzen 7 5800X 8-Core Processor".into()));
        assert_eq!(cpu_model("processor\t: 0\n"),None);
        assert_eq!(mem_total_mb("MemTotal:       32768000 kB\nMemFree: 1 kB\n"),Some(32000));
        assert_eq!(mem_total_mb(""),None);
    }
    #[test]
    fn os_release_reads_only_pretty_name_without_shell_expansion() {
        assert_eq!(pretty_name("NAME=wrong\nPRETTY_NAME=\"Example Linux 42 (日本語)\"\n"),Some("Example Linux 42 (日本語)".into()));
        assert_eq!(pretty_name("PRETTY_NAME='Example $(touch /tmp/no)'\n"),Some("Example $(touch /tmp/no)".into()));
        assert_eq!(pretty_name("NAME=Linux\nPRETTY_NAME=\"\"\n"),None);
    }
    #[cfg(not(windows))]
    #[tokio::test]
    async fn version_probe_is_bounded_and_rejects_failed_or_oversized_output() {
        use std::time::{Duration,Instant};
        let started=Instant::now();
        assert_eq!(version_output(tokio::process::Command::new("sh").args(["-c","exec sleep 30"]),Duration::from_millis(50)).await,None);
        assert!(started.elapsed()<Duration::from_secs(2));
        assert_eq!(version_output(tokio::process::Command::new("sh").args(["-c","printf 'wine-10.0\\n'"]),Duration::from_secs(1)).await,Some("wine-10.0".into()));
        assert_eq!(version_output(tokio::process::Command::new("sh").args(["-c","printf 'wine-10.0'; exit 1"]),Duration::from_secs(1)).await,None);
        assert_eq!(version_output(tokio::process::Command::new("sh").args(["-c","printf '%02000d' 0"]),Duration::from_secs(1)).await,None);
    }
}
