//! Bounded, display-only evidence for launcher failures. Full diagnostics stay unchanged.
#[derive(Clone, Copy)]
pub(super) enum Hint { General, Component, GameFolder }
pub(super) struct Presentation { pub summary:String, pub diagnosis:Option<String> }
pub(super) fn present(message:&str,hint:Hint,prefixes:&[(String,&str)])->Presentation {
    let mut prefixes=prefixes.to_vec();prefixes.sort_by_key(|(path,_)|std::cmp::Reverse(path.len()));
    let lines=message.lines().map(|line| {
        let mut line=yo3_tunnel::diagnostics::redact_secrets(line,&[]);
        line=line.replace(r"\\?\UNC\",r"\\").replace(r"\\?\","");
        for (path,label) in &prefixes {
            let path=path.replace(r"\\?\","");
            if matches!(path.as_str(),"."|"..") || path.trim_matches(['/', '\\']).is_empty()
                || (path.as_bytes().get(1)==Some(&b':') && path[2..].trim_matches(['/', '\\']).is_empty()) {continue}
            let relative_name=!path.contains(['/', '\\']) && path.as_bytes().get(1)!=Some(&b':');
            let mut from=0;
            while let Some(offset)=line[from..].to_ascii_lowercase().find(&path.to_ascii_lowercase()) {
                let at=from+offset;let end=at+path.len();
                let before=line[..at].chars().next_back();let after=line[end..].chars().next();
                let boundary=before.is_none_or(|c|c.is_whitespace() || matches!(c,'\''|'"'|'('| '['|'{'|'='|':'));
                let ending=if relative_name {after.is_some_and(|c|matches!(c,'/'|'\\'))}
                    else {after.is_none_or(|c|matches!(c,'/'|'\\'|'\''|'"'|'.'|','|':'|')') || c.is_whitespace())};
                if boundary && ending {
                    line.replace_range(at..end,label);from=at+label.len();
                } else {from=end;}
            }
        }
        yo3_tunnel::diagnostics::redact(&line,&[],None).split_whitespace().collect::<Vec<_>>().join(" ")
    }).filter(|line|!line.is_empty() && line!="[sensitive diagnostic line omitted]").collect::<Vec<_>>();
    let first=lines.first().map_or("The operation could not finish.",String::as_str);
    if let Some((_,summary,evidence))=lines.iter().filter_map(|line|native_evidence(line)).max_by_key(|(rank,_,_)|*rank) {
        return Presentation {summary,diagnosis:Some(compact(&evidence,260))}
    }
    let summary=match hint {
        Hint::Component=>"Required Revival files need repair.".into(),
        Hint::GameFolder=>"The game folder needs attention.".into(),
        Hint::General if first.starts_with("Server discovery failed")=>"Couldn't reach this server.".into(),
        Hint::General if first.starts_with("Connection check failed")=>"Couldn't verify this connection.".into(),
        Hint::General if first.starts_with("The game could not start")=>"The game could not start.".into(),
        Hint::General if first.starts_with("The game stopped unexpectedly")=>"The game stopped unexpectedly.".into(),
        Hint::General=>compact(first,100),
    };
    let cause=lines.iter().find(|line|line.contains("(os error ")).map(|line| {
        let at=line.rfind("(os error ").unwrap();let end=line[at..].find(')').map_or(line.len(),|n|at+n+1);
        let stage=["TLS handshake", "TCP connect", "transport write", "read server discovery"]
            .into_iter().rev().find(|stage|line.contains(stage));
        let network=matches!(hint,Hint::General) && (stage.is_some() || first.starts_with("Server discovery failed") || first.starts_with("Connection check failed"));
        if !network {
            let context=lines.join(" ").replace(&line[at..end],"");
            return format!("{} · {}",&line[at..end],context.trim())
        }
        let context=line[..at].rsplit(": ").next().unwrap_or(&line[..at]).trim();
        format!("{}{}: {}",stage.map(|stage|format!("{stage} · ")).unwrap_or_default(),&line[at..end],compact(context,180))
    }).unwrap_or_else(||lines.join(" "));
    let diagnosis=compact(&cause,260);
    let title_has_cause=matches!(hint,Hint::General) && lines.len()==1 && summary==first;
    Presentation {diagnosis:(!title_has_cause && !diagnosis.is_empty() && diagnosis!=summary).then_some(diagnosis),summary}
}
fn compact(text:&str,limit:usize)->String {
    let text=text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count()<=limit{return text}
    let end=text.char_indices().nth(limit-1).map_or(text.len(),|(at,_)|at);
    let prefix=&text[..end];let cut=prefix.rfind(char::is_whitespace).filter(|at|*at>prefix.len()/2).unwrap_or(prefix.len());
    format!("{}…",prefix[..cut].trim_end())
}
fn native_evidence(line:&str)->Option<(u8,String,String)> {
    let stage=line.split_whitespace().find_map(|part|part.strip_prefix("stage="))?.trim_end_matches(':');
    if !matches!(stage,"game-data-dir"|"runtime-path"|"runtime-image"|"runtime-manifest"|"runtime-dependency"
        |"inject-allocation"|"inject-loader"|"inject-write"|"inject-thread"|"inject-wait"|"inject-result"
        |"mod-load"|"mod-ready-thread"|"mod-ready-wait"|"mod-ready-result"|"mod-ready"|"mod-metadata"
        |"game-job"|"game-create"|"game-job-assign"|"game-before-resume"|"game-resume-control"|"game-resume"|"game-exit"
        |"mod-environment"|"mod-import-hooks"|"mod-login-hook"|"mod-starter-hook"|"mod-duel-hook") {return None}
    let number=|key:&str|line.split_whitespace().find_map(|part|part.strip_prefix(key)).and_then(|value| {
        let value=value.trim_end_matches(':');
        if let Some(hex)=value.strip_prefix("0x").or_else(||value.strip_prefix("0X")){u32::from_str_radix(hex,16).ok()}else{value.parse::<u32>().ok()}
    });
    let code=number("process_exit_code=").filter(|code|*code!=0);
    let windows=number("windows_error=").filter(|code|*code!=0);
    let mut evidence=format!("stage={stage}");let mut rank=1;
    let summary=if code==Some(0xC0000005) {evidence=format!("0xC0000005 · access violation · {evidence}");rank=4;"The game crashed."}
        else if let Some(code)=code {evidence=format!("Game exit code 0x{code:08X} · {evidence}");rank=4;"The game exited with an error."}
        else {
            if let Some(error)=windows {evidence=format!("Windows error {error} · {evidence}");rank=3;}
            if let (Some(status),Some(expected))=(number("mod_status="),number("expected_abi=")) {evidence.push_str(&format!(" · mod_status={status} · expected_abi={expected}"));rank=rank.max(2);}
            if number("remote_result=")==Some(0) {evidence.push_str(" · remote_result=0");rank=rank.max(2);}
            if line.split_whitespace().any(|part|part=="process_status=suspended") {evidence.push_str(" · process_status=suspended");}
            if let Some(result)=number("hresult=") {evidence.push_str(&format!(" · HRESULT 0x{result:08X}"));rank=rank.max(2);}
            if stage.starts_with("runtime-") {"The game runtime check failed."}else{"The game could not start."}
        };
    if code.is_none() && let Some((_,help))=line.split_once("stage=").and_then(|(_,native)|native.split_once(": ")) {
        evidence.push_str(": ");evidence.push_str(help);
    }
    Some((rank,summary.into(),evidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_evidence_keeps_the_filename_and_actual_missing_or_mismatch_reason() {
        let prefixes=vec![("/home/test/Revival".into(),"<app>")];
        let missing=present("Missing or invalid 32-bit injector: Required component not found: /home/test/Revival/yo3-auth-launch.exe. Repair the installation.",Hint::Component,&prefixes);
        let evidence=missing.diagnosis.unwrap();assert!(missing.summary.contains("Required Revival files"));
        assert!(evidence.contains("yo3-auth-launch.exe") && evidence.contains("not found"));assert!(!evidence.contains("/home/test"));
        let mismatch=present("Connection DLL validation failed: /home/test/Revival/yo3-auth.dll: Connection DLL release v0.0.5 does not match application v0.0.6. Repair with files from the same bundle.",Hint::Component,&prefixes);
        let evidence=mismatch.diagnosis.unwrap();assert!(evidence.contains("yo3-auth.dll") && evidence.contains("v0.0.5") && evidence.contains("does not match") && evidence.contains("v0.0.6"));
    }
    #[test]
    fn component_and_game_file_evidence_survives_a_nested_os_error_chain() {
        for hint in [Hint::Component,Hint::General] {
            let missing=present(r"Missing or invalid 32-bit injector: Required component not found: C:\Games\YO3 Revival\yo3-auth-launch.exe. Repair the installation or select the matching file.: The system cannot find the file specified. (os error 2)",hint,&[]).diagnosis.unwrap();
            assert!(missing.contains("yo3-auth-launch.exe") && missing.contains("not found") && missing.contains("os error 2"),"component cause was lost: {missing}");
            assert!(missing.chars().count()<=260);
        }
        let game=present(r"The game data is incomplete or unsupported: C:\Games\Yu-Gi-Oh! ONLINE 3\package\data002.ypk could not be read: The system cannot find the file specified. (os error 2)",Hint::GameFolder,&[]).diagnosis.unwrap();
        assert!(game.contains(r"package\data002.ypk") && game.contains("could not be read") && game.contains("os error 2"),"archive cause was lost: {game}");
        assert!(game.chars().count()<=260);
        let permission=present("Could not save ZIP: Permission denied (os error 13)",Hint::General,&[]);
        assert_eq!(permission.summary,"Could not save ZIP: Permission denied (os error 13)");assert!(permission.diagnosis.is_none());
    }
    #[test]
    fn native_game_exit_beats_the_outer_helper_status_and_survives_long_details() {
        let message=format!("The game could not start (exit status: 1).\n{}\nstage=game-exit process_exit_code=0xc0000005: Game exited with an error. Export logs for help.","Additional context. ".repeat(90));
        let display=present(&message,Hint::General,&[]);assert_eq!(display.summary,"The game crashed.");
        let evidence=display.diagnosis.unwrap();assert!(evidence.contains("0xC0000005") && evidence.contains("access violation") && evidence.contains("stage=game-exit"));
        assert!(!evidence.contains("exit status: 1") && !evidence.to_lowercase().contains("antivirus"));assert!(evidence.chars().count()<=260);
    }
    #[test]
    fn native_windows_errors_and_remote_loader_results_do_not_invent_causes() {
        let dependency=present("Game runtime check failed.\nRuntime checker (exit code: 1): stage=runtime-dependency windows_error=126: d3dx9_40.dll is missing or cannot load.\nstage=game-before-resume process_status=suspended",Hint::General,&[]).diagnosis.unwrap();
        assert!(dependency.contains("126") && dependency.contains("stage=runtime-dependency") && dependency.contains("d3dx9_40.dll"));
        let loader=present("stage=mod-load remote_result=0: Windows could not load or initialize the connection DLL.",Hint::General,&[]).diagnosis.unwrap();
        assert!(loader.contains("remote_result=0") && !loader.contains("Windows error"));
        let abi=present("stage=mod-ready windows_error=0 mod_status=1 expected_abi=2: Connection DLL did not report the required interface.",Hint::General,&[]).diagnosis.unwrap();
        assert!(abi.contains("mod_status=1") && abi.contains("expected_abi=2") && !abi.contains("Windows error 0"));
    }
    #[test]
    fn network_evidence_retains_nested_tls_and_os_error_without_blaming_credentials() {
        let display=present("Connection check failed: connecting to the host: TLS handshake: connection reset by peer (os error 10054)",Hint::General,&[]);
        assert!(display.summary.contains("connection"));let evidence=display.diagnosis.unwrap();
        assert!(evidence.contains("TLS handshake") && evidence.contains("10054") && evidence.contains("reset"));assert!(!evidence.contains("account key"));
        let long=present(&format!("Connection check failed: TLS handshake: {} connection reset (os error 10054)","Extra context. ".repeat(100)),Hint::General,&[]).diagnosis.unwrap();
        assert!(long.contains("10054") && long.contains("TLS handshake") && long.chars().count()<=260);
    }
    #[test]
    fn unknown_causes_are_unicode_safe_bounded_redacted_and_not_confused_with_native_fields() {
        let message=format!("Unknown import failure: {} stage=made-up process_exit_code=0xc0000005","坏😀 ".repeat(150));
        let display=present(&message,Hint::General,&[]);assert!(display.summary.chars().count()<=100);let evidence=display.diagnosis.unwrap();
        assert!(evidence.chars().count()<=260 && evidence.ends_with('…'));assert!(!evidence.contains("access violation"));
        let secret=present("Authentication failed\nkey=abcdefghijklmnopqrst\nConnection refused (os error 10061)",Hint::General,&[]);
        assert!(!format!("{} {:?}",secret.summary,secret.diagnosis).contains("abcdefghijklmnopqrst"));
        assert!(secret.diagnosis.unwrap().contains("10061"));
        let plain=present("Could not save a ZIP",Hint::General,&[]);assert_eq!(plain.summary,"Could not save a ZIP");assert!(plain.diagnosis.is_none());
        let prefixes=[".","..","/","\\","C:\\"].into_iter().map(|path|(path.into(),"<game>")).collect::<Vec<_>>();
        let punctuation=present("Could not open a file. Please retry.",Hint::General,&prefixes);
        assert_eq!(punctuation.summary,"Could not open a file. Please retry.");
        let relative=present("The game could not start: Game/yo3.exe is missing.",Hint::General,&[("Game".into(),"<game>")]);
        assert_eq!(relative.summary,"The game could not start.");
        let evidence=relative.diagnosis.unwrap();assert!(evidence.contains("The game could not start") && evidence.contains("<game>/yo3.exe"));
        let substring=present("NotGame/yo3.exe is missing.",Hint::General,&[("Game".into(),"<game>")]);
        assert_eq!(substring.summary,"NotGame/yo3.exe is missing.");
    }
}
