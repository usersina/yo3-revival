//! User-reviewed GitHub report text and validated evidence links.
use serde::{Deserialize, Serialize};
use crate::system_info::SystemInfo;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Context {
    pub connection: String,
    pub server_version: Option<String>,
    pub readiness: String,
    pub error: String,
    pub missing_files: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Draft { pub description: String, pub steps: String }

impl Draft {
    pub fn markdown(&self, context: &Context, system: &SystemInfo, zip_name: Option<&str>, report_ids: &[String]) -> String {
        let attachment=match zip_name {
            _ if !report_ids.is_empty()=>ids_sentence(report_ids),
            Some(name)=>format!("Attach {} to your GitHub issue by dragging the ZIP into its diagnostic logs field.",name.rsplit(['/', '\\']).find(|part|!part.is_empty()).unwrap_or("the diagnostic ZIP")),
            None=>"No diagnostic ZIP is attached; logs are unavailable or were excluded. You can submit this report without logs.".into(),
        };
        let report=format!("{}\n\n## Attachments\n\n{attachment}\n\nYou can also drag screenshots or a short video into the issue. Review the report before sharing it.\n",self.content_markdown(context,system));
        redact_report(&report)
    }
    fn content_markdown(&self, context: &Context, system: &SystemInfo) -> String {
        let captured=context.error.lines().map(str::trim).find(|line|!line.is_empty()).unwrap_or("Describe what went wrong.");
        let missing=if context.missing_files.is_empty(){"None detected.".into()}else{code_block(&context.missing_files.join("\n"))};
        let error=if context.error.trim().is_empty(){"No error was captured.".into()}else{code_block(&context.error)};
        format!("# YO3 Revival bug report\n\n## What happened?\n\n{}\n\n## Steps to reproduce\n\n{}\n\n## Application version\n\n{} ({})\n\n## System details\n\n{}\n\n## Connection\n\n{}\n\nServer version: {}\nSetup: {}\n\n## Current error\n\n{error}\n\n## Missing files\n\n{missing}",
            or_hint(&self.description,captured),or_hint(&self.steps,"Not provided."),
            yo3_tunnel::BUILD_VERSION,yo3_tunnel::BUILD_COMMIT,system_details(system),or_hint(&context.connection,"Not captured."),
            context.server_version.as_deref().unwrap_or("Not captured."),or_hint(&context.readiness,"Not captured."))
    }
    pub fn url(&self, context: &Context, system: &SystemInfo, zip_name: Option<&str>, report_ids: &[String]) -> anyhow::Result<String> {
        let mut url=reqwest::Url::parse("https://github.com/usersina/yo3-revival/issues/new")?;
        let mut logs=if !report_ids.is_empty(){ids_sentence(report_ids)}else if zip_name.is_some(){"Please attach the diagnostic ZIP you saved locally.".to_string()}else{"No diagnostic ZIP is attached; logs are unavailable or were excluded. You can submit this report without logs.".to_string()};
        if !context.error.trim().is_empty(){logs.push_str(if zip_name.is_some(){" The app captured an error; full details are in the local ZIP or Copy report."}else{" The app captured an error; use Copy report to review or paste its full details."});}
        if !context.missing_files.is_empty(){logs.push_str(&format!(" {} missing required files were detected; use Copy report for their exact paths.",context.missing_files.len()));}
        let native=native_evidence(&context.error);if !native.is_empty(){logs.push_str("\nNative diagnostics:\n");logs.push_str(&native);logs.push('\n');}
        logs.push_str(if report_ids.is_empty() { " Review files before attaching them; screenshots or a short video can help too." } else { " Screenshots or a short video can help too." });
        let description=query_text(&self.description);
        let title=description.lines().map(str::trim).find(|line|!line.is_empty()).unwrap_or("YO3 Revival problem").chars().take(100).collect::<String>();
        url.query_pairs_mut().append_pair("template","1-bug.yml").append_pair("title",&title)
            .append_pair("what",&description).append_pair("steps",&query_text(&self.steps))
            .append_pair("version",&format!("{} ({})",yo3_tunnel::BUILD_VERSION,yo3_tunnel::BUILD_COMMIT))
            .append_pair("pc",&query_text(&system_details(system))).append_pair("logs",&logs);
        anyhow::ensure!(url.as_str().len()<=7500,"This report is too long for a GitHub link (maximum 7500 encoded bytes). Choose Copy report, open the bug form and paste the report there. Attach a diagnostic ZIP if available.");
        Ok(url.into())
    }
}

/// The report IDs a report sent to the server got (problem reports): the host reads the logs by them.
fn ids_sentence(ids: &[String]) -> String {
    format!("Report ID{}: {}. The logs went to the server's host with {}; there is nothing to attach.",
        if ids.len() == 1 { "" } else { "s" }, ids.join(", "), if ids.len() == 1 { "it" } else { "them" })
}

fn or_hint<'a>(value:&'a str,hint:&'a str)->&'a str {if value.trim().is_empty(){hint}else{value}}
fn system_details(system:&SystemInfo)->String {
    let mut details=format!("OS: {}\nArchitecture: {}",system.os,system.arch);
    if let Some(wine)=&system.wine {details.push_str(&format!("\nWine: {wine}"));}
    details
}
fn code_block(text:&str)->String {
    let longest=text.split(|ch|ch!='`').map(str::len).max().unwrap_or(0);
    let fence="`".repeat(longest.max(2)+1);format!("{fence}text\n{text}\n{fence}")
}
fn redact_report(text:&str)->String {
    // User-written reports may exceed diagnostic line limits; keep their prose for Copy report.
    let mut private_key=false;
    text.lines().map(|line| {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----"){private_key=true;}
        let clean=if private_key{"[private key omitted]".into()}else{yo3_tunnel::diagnostics::redact_secrets(line,&[])};
        if line.contains("-----END") && line.contains("PRIVATE KEY-----"){private_key=false;}
        clean
    }).collect::<Vec<_>>().join("\n")
}
fn query_text(text:&str)->String {
    let home=std::env::var(if cfg!(windows){"USERPROFILE"}else{"HOME"}).ok();
    redact_report(text).lines().map(|line|yo3_tunnel::diagnostics::redact(line,&[],home.as_deref())).collect::<Vec<_>>().join("\n")
}
fn native_evidence(message:&str)->String {
    let mut evidence=Vec::new();
    for line in message.lines().rev() {
        if yo3_tunnel::diagnostics::is_trace(line) || line.contains("[tls]") || line.contains("] <-") || line.contains("] ->"){continue}
        let Some(stage)=line.split_whitespace().find_map(|part|part.strip_prefix("stage=")).map(|stage|stage.trim_end_matches(':')) else {continue};
        // Match the known native stages used by error_presentation; arbitrary stage text stays local.
        if !matches!(stage,"game-data-dir"|"runtime-path"|"runtime-image"|"runtime-manifest"|"runtime-dependency"|"runtime-import"
            |"inject-allocation"|"inject-loader"|"inject-write"|"inject-thread"|"inject-wait"|"inject-result"
            |"mod-load"|"mod-ready-thread"|"mod-ready-wait"|"mod-ready-result"|"mod-ready"|"mod-metadata"
            |"game-job"|"game-create"|"game-job-assign"|"game-startup"|"game-before-resume"|"game-resume-control"|"game-resume"|"game-exit"
            |"mod-environment"|"mod-import-hooks"|"mod-login-hook"|"mod-starter-hook"|"mod-duel-hook"){continue}
        let mut row=format!("stage={stage}");
        for key in ["windows_error","process_exit_code","mod_status"] {
            let prefix=format!("{key}=");let value=line.split_whitespace().find_map(|part|part.strip_prefix(&prefix)).map(|value|value.trim_end_matches(':'));
            if let Some((number,hex))=value.and_then(native_number) {
                row.push_str(&if key=="process_exit_code" && hex{format!(" {key}=0x{number:08X}")}else{format!(" {key}={number}")});
            }
        }
        if !evidence.contains(&row){evidence.push(row);if evidence.len()==4{break}}
    }
    evidence.join("\n")
}
fn native_number(value:&str)->Option<(u32,bool)> {
    if let Some(hex)=value.strip_prefix("0x").or_else(||value.strip_prefix("0X")) {
        if hex.is_empty() || hex.len()>8 || !hex.bytes().all(|byte|byte.is_ascii_hexdigit()){return None}
        u32::from_str_radix(hex,16).ok().map(|number|(number,true))
    }else{
        if value.is_empty() || value.len()>10 || !value.bytes().all(|byte|byte.is_ascii_digit()){return None}
        value.parse::<u32>().ok().map(|number|(number,false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn system() -> SystemInfo {SystemInfo{os:"Windows 11 (build 22631)".into(),arch:"x86_64".into(),wine:None,..Default::default()}}
    fn values(url: &str) -> std::collections::HashMap<String,String> {
        reqwest::Url::parse(url).unwrap().query_pairs().map(|(key,value)|(key.into_owned(),value.into_owned())).collect()
    }
    #[test]
    fn github_draft_encodes_unicode_and_query_delimiters_without_changing_the_fields() {
        let draft=Draft {description:"Stardust & 星 / #4? loses its effect".into(),steps:"1. Summon → battle\n2. Choose A&B / #duel".into()};
        let url=draft.url(&Context::default(),&system(),None,&[]).unwrap();let parsed=reqwest::Url::parse(&url).unwrap();let query=values(&url);
        assert_eq!(parsed.scheme(),"https");assert_eq!(parsed.host_str(),Some("github.com"));assert_eq!(parsed.path(),"/usersina/yo3-revival/issues/new");assert!(parsed.fragment().is_none());
        assert_eq!(query["template"],"1-bug.yml");assert_eq!(query["what"],draft.description);assert_eq!(query["steps"],draft.steps);
        assert_eq!(query.len(),7);assert!(url.len()<=7500);
    }
    #[test]
    fn github_title_uses_the_first_nonempty_scrubbed_line_and_limits_unicode_characters() {
        let draft=Draft {description:format!("\n  \n  Cannot play /home/Private/game: {}  \nA separate detail", "星🌟".repeat(70)),steps:String::new()};
        let url=draft.url(&Context::default(),&system(),None,&[]).unwrap();let query=values(&url);let title=&query["title"];
        let expected=format!("Cannot play /home/<user>/game: {}","星🌟".repeat(70)).chars().take(100).collect::<String>();
        assert_eq!(title,&expected);assert_eq!(title.chars().count(),100);assert!(!title.contains("Private"));assert!(!title.contains("separate detail"));
        let fallback=values(&Draft::default().url(&Context::default(),&system(),None,&[]).unwrap());assert_eq!(fallback["title"],"YO3 Revival problem");
        assert!(fallback["logs"].contains("No diagnostic ZIP is attached"));assert!(!fallback["logs"].contains("save or export"));
        assert!(draft.markdown(&Context::default(),&system(),None,&[]).contains("No diagnostic ZIP is attached"));
    }
    #[test]
    fn github_query_ids_match_the_actual_bug_form_and_only_description_is_required() {
        let template=include_str!("../../../.github/ISSUE_TEMPLATE/1-bug.yml");
        let query=values(&Draft {description:"The game stopped".into(),steps:String::new()}.url(&Context::default(),&system(),None,&[]).unwrap());
        for field in ["what","steps","version","pc","logs"] {assert!(template.contains(&format!("\n    id: {field}\n")),"query field {field} is not in the real GitHub form");assert!(query.contains_key(field));}
        assert!(template.contains("  - type: textarea\n    id: pc\n"));assert!(template.contains("  - type: textarea\n    id: steps\n"));
        assert_eq!(template.matches("required: true").count(),1);assert!(template.contains("label: Screenshot or video"));
    }
    #[test]
    fn github_url_excludes_full_diagnostics_paths_and_credentials() {
        let secret="ABCDEFGHIJKLMNOPQRST";
        let context=Context {connection:"Friends at 203.0.113.19:7443".into(),server_version:Some("v0.0.6".into()),readiness:"Missing C:\\Users\\Private\\game\\yo3.exe".into(),
            error:format!("[YO3 PACKET] raw-packet-only\nFailed at /home/private/game/mod.dll\nkey={secret}"),missing_files:vec!["C:\\Users\\Private\\app\\yo3-auth.dll".into()]};
        let draft=Draft {description:format!("My player cannot summon\nPath /home/Private/game/yo3.exe\nkey={secret}"),steps:"Choose player, then duel".into()};
        let url=draft.url(&context,&system(),Some("C:\\Users\\Private\\Downloads\\report.zip"),&[]).unwrap();let query=values(&url);let decoded=query.values().cloned().collect::<Vec<_>>().join("\n");
        for sensitive in [secret,"203.0.113.19","raw-packet-only","C:\\Users\\Private","/home/Private","/home/private","mod.dll","yo3-auth.dll"] {assert!(!decoded.contains(sensitive),"private diagnostic leaked into URL: {sensitive}");}
        assert!(query["what"].contains("My player cannot summon"));assert_eq!(query["steps"],draft.steps);
        assert!(query["logs"].contains("attach"));
    }
    #[test]
    fn native_evidence_in_the_github_draft_is_bounded_and_never_copies_raw_error_data() {
        let context=Context {error:concat!(
            "Native failure at C:\\Users\\Private\\game\\yo3.exe on 203.0.113.19 key=ABCDEFGHIJKLMNOPQRST\n",
            "stage=private-stage process_exit_code=0xc0000005\n",
            "[YO3 PACKET] stage=mod-load windows_error=10061 original-packet-only\n",
            "stage=game-exit process_exit_code=0xc0000005 windows_error=5 mod_status=2: full-private-error\n",
            "stage=mod-ready windows_error=4294967296 process_exit_code=0x100000000 mod_status=secret-status\n"
        ).into(),..Context::default()};
        let draft=Draft {description:"The game crashes when opening".into(),steps:String::new()};
        let url=draft.url(&context,&system(),None,&[]).unwrap();let query=values(&url);let logs=&query["logs"];
        for expected in ["stage=game-exit","process_exit_code=0xC0000005","windows_error=5","mod_status=2","Copy report"] {assert!(logs.contains(expected),"native evidence missing: {expected}");}
        for private in ["C:\\Users", "Private", "203.0.113.19", "ABCDEFGHIJKLMNOPQRST", "private-stage", "original-packet-only", "10061", "full-private-error", "4294967296", "0x100000000", "secret-status"] {assert!(!logs.contains(private),"raw or invalid error data leaked: {private}");}
        let repeated=Context {error:"stage=game-exit windows_error=5 process_exit_code=0xc0000005\n".repeat(1000),..Context::default()};
        let repeated=values(&draft.url(&repeated,&system(),None,&[]).unwrap());assert!(repeated["logs"].len()<1000,"repeated native lines must not grow the URL without bound");
    }
    #[test]
    fn a_report_id_takes_the_place_of_the_logs_file() {
        let ids=vec!["R-7K3Q-M2PA".to_string()];
        let draft=Draft {description:"The shop disconnected".into(),steps:String::new()};
        let logs=&values(&draft.url(&Context::default(),&system(),Some("report.zip"),&ids).unwrap())["logs"];
        assert!(logs.contains("Report ID: R-7K3Q-M2PA") && logs.contains("nothing to attach") && !logs.contains("ZIP"), "{logs}");
        let report=draft.markdown(&Context::default(),&system(),None,&ids);
        assert!(report.contains("Report ID: R-7K3Q-M2PA") && !report.contains("Attach "));
        let two=values(&draft.url(&Context::default(),&system(),None,&["R-AAAA-BBBB".into(),"R-CCCC-DDDD".into()]).unwrap());
        assert!(two["logs"].contains("Report IDs: R-AAAA-BBBB, R-CCCC-DDDD"));
    }
    #[test]
    fn overlong_encoded_urls_offer_copy_report_instead_of_truncating_user_text() {
        let draft=Draft {description:"界".repeat(1000),steps:"Return to the menu".into()};
        let error=draft.url(&Context::default(),&system(),None,&[]).unwrap_err().to_string();assert!(error.contains("Copy report"));assert!(error.contains("7500"));
        let steps=Draft {description:"The game failed".into(),steps:"Repeat step. ".repeat(2000)};
        assert!(steps.url(&Context::default(),&system(),None,&[]).unwrap_err().to_string().contains("Copy report"));
        assert!(steps.markdown(&Context::default(),&system(),None,&[]).contains(&steps.steps),"the full local report must retain long user text");
    }
    #[test]
    fn markdown_keeps_full_context_and_requests_manual_attachments_without_secrets() {
        let context=Context {connection:"Friends at 203.0.113.19:7443".into(),server_version:Some("v0.0.6".into()),readiness:"Connection components missing".into(),
            error:"Injection failed: Windows error 126 at load_mod\nkey=ABCDEFGHIJKLMNOPQRST".into(),missing_files:vec!["C:\\Games\\Revival\\yo3-auth.dll".into(),"C:\\Games\\YO3\\yo3.idx".into()]};
        let draft=Draft {description:"The game closes".into(),steps:"1. Play\n2. Open ONLINE".into()};
        let report=draft.markdown(&context,&system(),Some("C:\\Users\\Private\\Downloads\\yo3-report.zip"),&[]);
        for included in ["What happened","Steps to reproduce","System details","Connection","Current error","Missing files","Attachments","The game closes","1. Play","Windows 11","x86_64","203.0.113.19","v0.0.6","Windows error 126","load_mod","yo3-auth.dll","yo3.idx","yo3-report.zip"] {assert!(report.contains(included),"missing report detail: {included}");}
        assert!(!report.contains("ABCDEFGHIJKLMNOPQRST"));assert!(!report.contains("C:\\Users\\Private\\Downloads"));
    }
    #[test]
    fn markdown_scrubs_multiline_private_keys_and_includes_wine_when_present() {
        let context=Context {error:"Before\n-----BEGIN PRIVATE KEY-----\nsmall/secret+parts=\n-----END PRIVATE KEY-----\nAfter".into(),..Context::default()};
        let system=SystemInfo{os:"Linux".into(),arch:"x86_64".into(),wine:Some("wine-10.0".into()),..Default::default()};
        let report=Draft::default().markdown(&context,&system,None,&[]);assert!(report.contains("wine-10.0"));assert!(report.contains("Before"));assert!(report.contains("After"));assert!(!report.contains("small/secret+parts="));
    }

    #[test]
    fn publishing_draft_and_system_details_survive_persistence_round_trip() {
        let draft=Draft {description:"Stardust 星".into(),steps:"Summon, then battle".into()};
        let restored:Draft=serde_json::from_str(&serde_json::to_string(&draft).unwrap()).unwrap();
        assert_eq!(restored.description,draft.description);assert_eq!(restored.steps,draft.steps);
        let system=SystemInfo {wine:Some("wine-10.0".into()),..system()};
        let restored:SystemInfo=serde_json::from_str(&serde_json::to_string(&system).unwrap()).unwrap();
        assert_eq!(restored.os,system.os);assert_eq!(restored.arch,system.arch);assert_eq!(restored.wine,system.wine);
    }
}
