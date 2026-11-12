//! GitHub App device authorization and explicit report publishing against fixed GitHub endpoints.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde_json::{Value, json};

const API: &str = "https://api.github.com/";
const OAUTH: &str = "https://github.com/";
const MAX_BODY: usize = 8 * 1024 * 1024;
const MAX_RESPONSE: usize = 10 * 1024 * 1024;
const PAGE_SIZE: usize = 30;
const MAX_PAGES: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Authorize, Poll, Refresh, Identity, Gist, FindGist, FindIssue, Publish }
#[derive(Clone, Debug)]
pub struct Failure { pub stage: Stage, pub message: String, pub uncertain: bool, pub auth_expired: bool }
impl std::fmt::Display for Failure {fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str(&self.message)}}
impl std::error::Error for Failure {}
impl Failure {
    fn new(stage:Stage,message:impl Into<String>)->Self {Self {stage,message:message.into(),uncertain:false,auth_expired:false}}
    fn unknown(stage:Stage,message:impl Into<String>)->Self {Self {uncertain:true,..Self::new(stage,message)}}
}
#[derive(Clone)]
pub struct Session {pub user_code:String,pub verification_uri:String,pub expires_in:u64,pub interval:u64,client_id:String,device_code:String,started:Instant}
impl std::fmt::Debug for Session {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("Session").field("expires_in",&self.expires_in).field("interval",&self.interval).finish_non_exhaustive()}}
#[derive(Clone)]
pub struct Token {access_token:String,expires_at:Option<u64>,refresh_token:Option<String>,refresh_expires_at:Option<u64>}
impl std::fmt::Debug for Token {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("Token").field("expires_at",&self.expires_at).field("has_refresh",&self.refresh_token.is_some()).finish_non_exhaustive()}}
impl Token {
    pub fn new(access_token:String)->Result<Self,Failure> {Self::from_secret(access_token,None)}
    pub fn from_secret(access_token:String,expires_at:Option<u64>)->Result<Self,Failure> {Self::from_secrets(access_token,expires_at,None,None)}
    pub fn from_secrets(access_token:String,expires_at:Option<u64>,refresh_token:Option<String>,refresh_expires_at:Option<u64>)->Result<Self,Failure> {
        if !valid_secret(&access_token) || refresh_token.as_ref().is_some_and(|secret|!valid_secret(secret)){return Err(Failure::new(Stage::Identity,"Saved GitHub credentials are invalid. Sign in again."))}
        Ok(Self {access_token,expires_at,refresh_token,refresh_expires_at})
    }
    pub fn access_token(&self)->&str {&self.access_token}
    pub fn expires_at(&self)->Option<u64> {self.expires_at}
    pub fn refresh_token(&self)->Option<&str> {self.refresh_token.as_deref()}
    pub fn refresh_expires_at(&self)->Option<u64> {self.refresh_expires_at}
}
#[derive(Clone, Debug)]
pub struct Identity {pub login:String,pub url:String}
#[derive(Clone, Debug)]
pub struct Gist {pub id:String,pub url:String}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {pub number:u64,pub url:String}
#[derive(Clone)]
pub struct Client {http:reqwest::Client,api:reqwest::Url,oauth:reqwest::Url,poll_unit:Duration}

pub fn configured_client_id()->Option<String> {
    std::env::var("YO3_GITHUB_CLIENT_ID").ok().filter(|value|!value.trim().is_empty())
        .or_else(||option_env!("YO3_GITHUB_CLIENT_ID").map(str::to_owned).filter(|value|!value.trim().is_empty())).or_else(||Some("Iv23liaeJfzPCYXyBJEj".into())).map(|value|value.trim().to_owned())
}
impl Client {
    pub fn new()->Result<Self,Failure> {Self::build(API,OAUTH,Duration::from_secs(20),Duration::from_secs(1),true)}
    fn build(api:&str,oauth:&str,timeout:Duration,poll_unit:Duration,https_only:bool)->Result<Self,Failure> {
        let _ = yo3_tunnel::tokio_rustls::rustls::crypto::ring::default_provider().install_default();
        let http=reqwest::Client::builder().user_agent("YO3-Revival-Reports")
            .redirect(reqwest::redirect::Policy::none()).retry(reqwest::retry::never()).https_only(https_only)
            .connect_timeout(Duration::from_secs(8)).timeout(timeout).build().map_err(|_|Failure::new(Stage::Authorize,"Could not initialize the secure GitHub connection."))?;
        Ok(Self {http,api:reqwest::Url::parse(api).map_err(|_|Failure::new(Stage::Authorize,"Invalid GitHub endpoint."))?,oauth:reqwest::Url::parse(oauth).map_err(|_|Failure::new(Stage::Authorize,"Invalid GitHub endpoint."))?,poll_unit})
    }
    pub async fn start_auth(&self,client_id:&str)->Result<Session,Failure> {
        validate_client_id(client_id,Stage::Authorize)?;
        let response=self.oauth("login/device/code",&[("client_id",client_id)],Stage::Authorize).await?;
        if let Some(error)=response.get("error").and_then(Value::as_str) {return Err(oauth_failure(Stage::Authorize,error))}
        let invalid=||Failure::new(Stage::Authorize,"GitHub returned an invalid sign-in code. Try signing in again.");
        let device_code=response.get("device_code").and_then(Value::as_str).filter(|value|valid_secret(value)).ok_or_else(invalid)?;
        let user_code=response.get("user_code").and_then(Value::as_str).filter(|value|!value.is_empty() && value.len()<=64 && value.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'-')).ok_or_else(invalid)?;
        if response.get("verification_uri").and_then(Value::as_str)!=Some("https://github.com/login/device"){return Err(invalid())}
        let expires_in=response.get("expires_in").and_then(Value::as_u64).filter(|value|(1..=3600).contains(value)).ok_or_else(invalid)?;
        let interval=response.get("interval").and_then(Value::as_u64).filter(|value|(1..=60).contains(value)).ok_or_else(invalid)?;
        Ok(Session {user_code:user_code.into(),verification_uri:"https://github.com/login/device".into(),expires_in,interval,client_id:client_id.into(),device_code:device_code.into(),started:Instant::now()})
    }
    pub async fn poll_auth(&self,session:&Session,cancel:&AtomicBool)->Result<Token,Failure> {
        if !(1..=3600).contains(&session.expires_in) || !(1..=60).contains(&session.interval){return Err(oauth_failure(Stage::Poll,"expired_token"))}
        let deadline=session.started+self.poll_unit*session.expires_in as u32;
        let mut interval=session.interval;
        loop {
            let wait=self.poll_unit*interval as u32;
            tokio::select! {
                biased;
                _=self.cancelled(cancel)=>return Err(Failure::new(Stage::Poll,"GitHub sign-in was cancelled.")),
                _=tokio::time::sleep_until(deadline.into())=>return Err(oauth_failure(Stage::Poll,"expired_token")),
                _=tokio::time::sleep(wait)=>{}
            }
            let fields=[("client_id",session.client_id.as_str()),("device_code",session.device_code.as_str()),("grant_type","urn:ietf:params:oauth:grant-type:device_code")];
            let response=tokio::select! {
                biased;
                _=self.cancelled(cancel)=>return Err(Failure::new(Stage::Poll,"GitHub sign-in was cancelled.")),
                _=tokio::time::sleep_until(deadline.into())=>return Err(oauth_failure(Stage::Poll,"expired_token")),
                response=self.oauth("login/oauth/access_token",&fields,Stage::Poll)=>response?
            };
            match response.get("error").and_then(Value::as_str) {
                Some("authorization_pending")=>{},
                Some("slow_down")=>{interval=(interval+5).max(response.get("interval").and_then(Value::as_u64).unwrap_or(0)).min(300);},
                Some(error)=>return Err(oauth_failure(Stage::Poll,error)),
                None=>return parse_token(&response,Stage::Poll)
            }
        }
    }
    pub async fn refresh_token(&self,client_id:&str,token:&Token)->Result<Token,Failure> {
        validate_client_id(client_id,Stage::Refresh)?;
        let refresh=token.refresh_token().filter(|_|!expired(token.refresh_expires_at())).ok_or_else(||expired_failure(Stage::Refresh))?;
        let response=self.oauth("login/oauth/access_token",&[("client_id",client_id),("grant_type","refresh_token"),("refresh_token",refresh)],Stage::Refresh).await?;
        if let Some(error)=response.get("error").and_then(Value::as_str){return Err(oauth_failure(Stage::Refresh,error))}
        let result=parse_token(&response,Stage::Refresh)?;
        if result.refresh_token().is_none(){return Err(expired_failure(Stage::Refresh))}
        Ok(result)
    }
    pub async fn identity(&self,token:&Token)->Result<Identity,Failure> {
        let response=self.api_request(reqwest::Method::GET,"user",token,None,Stage::Identity).await?;
        let login=response.get("login").and_then(Value::as_str).filter(|value|valid_login(value)).ok_or_else(||Failure::new(Stage::Identity,"GitHub returned an invalid account. Sign in again."))?;
        Ok(Identity {login:login.into(),url:format!("https://github.com/{login}")})
    }
    pub async fn upload_gist(&self,token:&Token,files:&BTreeMap<String,String>,report_id:&str)->Result<Gist,Failure> {
        let marker=report_marker(report_id,Stage::Gist)?;
        if files.is_empty() || files.len()+usize::from(!files.contains_key("README.md"))>100 || files.keys().any(|name|!valid_filename(name)) {return Err(Failure::new(Stage::Gist,"The diagnostic file list is invalid. Export the report locally instead."))}
        let mut files=files.iter().map(|(name,content)|(name.clone(),json!({"content":content}))).collect::<BTreeMap<_,_>>();
        let readme=files.entry("README.md".into()).or_insert_with(||json!({"content":"YO3 Revival diagnostic report. This unlisted link is included in the public bug report."}));
        let content=readme["content"].as_str().unwrap_or_default();
        readme["content"]=Value::String(format!("{content}\n\n{marker}\n"));
        let body=json!({"description":format!("YO3 Revival diagnostics {marker}"),"public":false,"files":files});
        let response=self.api_request(reqwest::Method::POST,"gists",token,Some(body),Stage::Gist).await?;
        parse_gist(&response,Stage::Gist,true)
    }
    pub async fn find_gist(&self,token:&Token,report_id:&str)->Result<Option<Gist>,Failure> {
        let marker=report_marker(report_id,Stage::FindGist)?;
        for page in 1..=MAX_PAGES {
            let response=self.api_request(reqwest::Method::GET,&format!("gists?per_page={PAGE_SIZE}&page={page}"),token,None,Stage::FindGist).await?;
            let list=response.as_array().ok_or_else(||Failure::new(Stage::FindGist,"GitHub returned an invalid diagnostic lookup. Do not resend until the earlier upload is resolved."))?;
            for item in list {
                if item.get("description").and_then(Value::as_str).is_some_and(|value|value.contains(&marker)) {
                    let gist=parse_gist(item,Stage::FindGist,false)?;
                    let detail=self.api_request(reqwest::Method::GET,&format!("gists/{}",gist.id),token,None,Stage::FindGist).await?;
                    if detail.pointer("/files/README.md/content").and_then(Value::as_str).is_some_and(|value|value.lines().any(|line|line.trim()==marker)){return Ok(Some(gist))}
                }
            }
            if list.len()<PAGE_SIZE{return Ok(None)}
        }
        Err(Failure::new(Stage::FindGist,"The earlier diagnostic upload could not be ruled out. Check your GitHub Gists before trying again."))
    }
    pub async fn find_issue(&self,token:&Token,report_id:&str)->Result<Option<Published>,Failure> {
        let marker=report_marker(report_id,Stage::FindIssue)?;
        let identity=self.identity(token).await.map_err(|mut error|{error.stage=Stage::FindIssue;error})?;
        for page in 1..=MAX_PAGES {
            let path=format!("repos/usersina/yo3-revival/issues?creator={}&state=all&sort=created&direction=desc&per_page={PAGE_SIZE}&page={page}",identity.login);
            let response=self.api_request(reqwest::Method::GET,&path,token,None,Stage::FindIssue).await?;
            let list=response.as_array().ok_or_else(||Failure::new(Stage::FindIssue,"GitHub returned an invalid report lookup. Do not resend until the earlier report is resolved."))?;
            for item in list {
                if item.get("pull_request").is_some(){continue}
                if item.pointer("/user/login").and_then(Value::as_str)!=Some(&identity.login){continue}
                if item.get("body").and_then(Value::as_str).is_some_and(|value|value.lines().any(|line|line.trim()==marker)){return parse_published(item,Stage::FindIssue,false).map(Some)}
            }
            if list.len()<PAGE_SIZE{return Ok(None)}
        }
        Err(Failure::new(Stage::FindIssue,"The earlier report could not be ruled out. Check your GitHub issues before trying again."))
    }
    pub async fn publish_issue(&self,token:&Token,title:&str,body:&str,report_id:&str)->Result<Published,Failure> {
        let marker=report_marker(report_id,Stage::Publish)?;
        if title.trim().is_empty() || title.chars().count()>256 || title.chars().any(char::is_control) || body.len()>64*1024 {return Err(Failure::new(Stage::Publish,"The report is too long or has an invalid title. Shorten it and try again."))}
        let body=if body.lines().any(|line|line.trim()==marker){body.to_owned()}else{format!("{body}\n\n{marker}")};
        if body.len()>64*1024{return Err(Failure::new(Stage::Publish,"The report is too long. Shorten it and try again."))}
        let response=self.api_request(reqwest::Method::POST,"repos/usersina/yo3-revival/issues",token,Some(json!({"title":title,"body":body})),Stage::Publish).await?;
        parse_published(&response,Stage::Publish,true)
    }
    async fn cancelled(&self,cancel:&AtomicBool) {
        while !cancel.load(Ordering::Acquire){tokio::time::sleep(self.poll_unit.min(Duration::from_millis(50))).await;}
    }
    async fn oauth(&self,path:&str,fields:&[(&str,&str)],stage:Stage)->Result<Value,Failure> {
        let mut encoded=reqwest::Url::parse("https://github.com/").expect("fixed URL");
        encoded.query_pairs_mut().extend_pairs(fields.iter().copied());
        let body=encoded.query().unwrap_or_default().to_owned();
        let url=self.oauth.join(path).map_err(|_|Failure::new(stage,"Invalid GitHub sign-in endpoint."))?;
        let request=self.http.post(url).header("Accept","application/json").header("Content-Type","application/x-www-form-urlencoded").body(body);
        self.send(request,stage,false,true).await
    }
    async fn api_request(&self,method:reqwest::Method,path:&str,token:&Token,body:Option<Value>,stage:Stage)->Result<Value,Failure> {
        if expired(token.expires_at()){return Err(expired_failure(stage))}
        let mutation=method==reqwest::Method::POST;
        let url=self.api.join(path).map_err(|_|Failure::new(stage,"Invalid GitHub API endpoint."))?;
        let mut request=self.http.request(method,url).bearer_auth(token.access_token()).header("Accept","application/vnd.github+json").header("X-GitHub-Api-Version","2026-03-10");
        if let Some(body)=body {
            let bytes=serde_json::to_vec(&body).map_err(|_|Failure::new(stage,"Could not prepare the GitHub report."))?;
            if bytes.len()>MAX_BODY{return Err(Failure::new(stage,"The diagnostic report exceeds the 8 MiB upload limit. Export logs locally instead."))}
            request=request.header("Content-Type","application/json").body(bytes);
        }
        self.send(request,stage,mutation,false).await
    }
    async fn send(&self,request:reqwest::RequestBuilder,stage:Stage,mutation:bool,oauth_errors:bool)->Result<Value,Failure> {
        let failure=|message:&str,uncertain:bool|if uncertain {Failure::unknown(stage,message)}else{Failure::new(stage,message)};
        let mut response=request.send().await.map_err(|_|failure("GitHub did not confirm the request. Check your connection before trying again.",mutation))?;
        let status=response.status();
        if status==reqwest::StatusCode::UNAUTHORIZED {return Err(expired_failure(stage))}
        let oauth_error_status=oauth_errors && status==reqwest::StatusCode::BAD_REQUEST;
        if !status.is_success() && !oauth_error_status {
            let message=match status.as_u16(){403=>"GitHub denied permission. Check the app authorization and repository access.",404=>"GitHub could not find the reporting integration. Check its installation on this repository.",429=>"GitHub is limiting requests. Wait before trying again.",422=>"GitHub rejected the report. Check its contents before trying again.",_ if status.is_redirection()=>"GitHub redirected the request unexpectedly. No credentials were forwarded.",_ if status.is_server_error()=>"GitHub is temporarily unavailable. Check whether the report was created before trying again.",_=>"GitHub rejected the request. Try again later."};
            return Err(failure(message,mutation && (status.is_server_error() || status.is_redirection())));
        }
        let uncertain=mutation && status.is_success();
        if response.content_length().is_some_and(|length|length>MAX_RESPONSE as u64){return Err(failure("GitHub returned an oversized response. The result could not be confirmed.",uncertain))}
        let mut bytes=Vec::new();
        while let Some(chunk)=response.chunk().await.map_err(|_|failure("GitHub's response was interrupted. The result could not be confirmed.",uncertain))? {
            if bytes.len().saturating_add(chunk.len())>MAX_RESPONSE{return Err(failure("GitHub returned an oversized response. The result could not be confirmed.",uncertain))}
            bytes.extend_from_slice(&chunk);
        }
        let value:Value=serde_json::from_slice(&bytes).map_err(|_|failure("GitHub returned an unreadable response. The result could not be confirmed.",uncertain))?;
        if oauth_error_status && value.get("error").and_then(Value::as_str).is_none(){return Err(Failure::new(stage,"GitHub rejected the sign-in request. Start sign-in again."))}
        Ok(value)
    }

}
fn valid_secret(value:&str)->bool {!value.is_empty() && value.len()<=4096 && value.bytes().all(|byte|byte.is_ascii_alphanumeric() || b"_-./+~".contains(&byte))}

fn now()->u64 {SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
fn expired(value:Option<u64>)->bool {value.is_some_and(|value|value<=now())}
fn expired_failure(stage:Stage)->Failure {Failure {auth_expired:true,..Failure::new(stage,"Your GitHub sign-in has expired or was revoked. Sign in again.")}}
fn validate_client_id(value:&str,stage:Stage)->Result<(),Failure> {
    if value.is_empty() || value.len()>128 || !value.bytes().all(|byte|byte.is_ascii_alphanumeric() || b"._-".contains(&byte)){return Err(Failure::new(stage,"GitHub reporting is not configured. Use the GitHub issue page instead."))}Ok(())
}
fn oauth_failure(stage:Stage,error:&str)->Failure {
    match error {
        "access_denied"=>Failure::new(stage,"GitHub sign-in was declined. You can sign in again when ready."),
        "expired_token"|"token_expired"|"bad_refresh_token"=>expired_failure(stage),
        "device_flow_disabled"|"incorrect_client_credentials"=>Failure::new(stage,"GitHub reporting is not configured correctly. Use the GitHub issue page instead."),
        "incorrect_device_code"|"bad_verification_code"=>Failure::new(stage,"The GitHub sign-in code is no longer valid. Start sign-in again."),
        _=>Failure::new(stage,"GitHub could not complete sign-in. Start sign-in again.")
    }
}
fn parse_token(value:&Value,stage:Stage)->Result<Token,Failure> {
    let invalid=||Failure::new(stage,"GitHub returned invalid sign-in credentials. Sign in again.");
    if value.get("token_type").and_then(Value::as_str).is_some_and(|kind|!kind.eq_ignore_ascii_case("bearer")){return Err(invalid())}
    let access=value.get("access_token").and_then(Value::as_str).filter(|secret|valid_secret(secret)).ok_or_else(invalid)?;
    let refresh=match value.get("refresh_token"){Some(Value::String(secret)) if valid_secret(secret)=>Some(secret.clone()),None=>None,_=>return Err(invalid())};
    let expiration=|key:&str|->Result<Option<u64>,Failure>{match value.get(key){None=>Ok(None),Some(value)=>value.as_u64().filter(|seconds|(1..=366*24*3600).contains(seconds)).and_then(|seconds|now().checked_add(seconds)).map(Some).ok_or_else(invalid)}};
    Ok(Token {access_token:access.into(),expires_at:expiration("expires_in")?,refresh_token:refresh,refresh_expires_at:expiration("refresh_token_expires_in")?})
}
fn valid_login(value:&str)->bool {!value.is_empty() && value.len()<=39 && value.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'-')}
fn valid_filename(value:&str)->bool {!value.is_empty() && value.len()<=160 && value!="." && value!=".." && !value.chars().any(|ch|ch.is_control() || matches!(ch,'/'|'\\'|':'))}
fn report_marker(report_id:&str,stage:Stage)->Result<String,Failure> {
    if !(8..=80).contains(&report_id.len()) || !report_id.bytes().all(|byte|byte.is_ascii_alphanumeric() || b"_-".contains(&byte)){return Err(Failure::new(stage,"The report identifier is invalid. Prepare a new report."))}Ok(format!("<!-- yo3-report:{report_id} -->"))
}
fn parse_gist(value:&Value,stage:Stage,uncertain:bool)->Result<Gist,Failure> {
    let id=value.get("id").and_then(Value::as_str).filter(|id|!id.is_empty() && id.len()<=80 && id.bytes().all(|byte|byte.is_ascii_hexdigit())).ok_or_else(||Failure {uncertain,..Failure::new(stage,"GitHub did not confirm the diagnostic link. Check your Gists before trying again.")})?;
    Ok(Gist {id:id.into(),url:format!("https://gist.github.com/{id}")})
}
fn parse_published(value:&Value,stage:Stage,uncertain:bool)->Result<Published,Failure> {
    let number=value.get("number").and_then(Value::as_u64).filter(|number|*number>0).ok_or_else(||Failure {uncertain,..Failure::new(stage,"GitHub did not confirm the report number. Check your issues before trying again.")})?;
    Ok(Published {number,url:format!("https://github.com/usersina/yo3-revival/issues/{number}")})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc,Mutex};
    use tokio::io::{AsyncReadExt,AsyncWriteExt};
    #[derive(Clone)]
    struct Request {target:String,headers:BTreeMap<String,String>,body:Vec<u8>}
    struct Reply {status:u16,body:String,headers:Vec<(String,String)>,drop_connection:bool,delay:Duration}
    fn reply(body:Value)->Reply {Reply {status:200,body:body.to_string(),headers:vec![],drop_connection:false,delay:Duration::ZERO}}
    fn created(body:Value)->Reply {Reply {status:201,..reply(body)}}
    async fn mock(replies:Vec<Reply>)->(Client,Arc<Mutex<Vec<Request>>>,tokio::task::JoinHandle<()>) {
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let base=format!("http://{}/",listener.local_addr().unwrap());
        let requests=Arc::new(Mutex::new(Vec::new()));let captured=requests.clone();
        let task=tokio::spawn(async move {for reply in replies {
            let (mut socket,_)=listener.accept().await.unwrap();let mut bytes=Vec::new();let mut part=[0;8192];
            let end=loop {let n=socket.read(&mut part).await.unwrap();if n==0{return}bytes.extend_from_slice(&part[..n]);if let Some(at)=bytes.windows(4).position(|part|part==b"\r\n\r\n"){break at+4}};
            let head=String::from_utf8(bytes[..end].to_vec()).unwrap();let mut lines=head.lines();let target=lines.next().unwrap().split_whitespace().nth(1).unwrap().to_owned();
            let headers=lines.filter_map(|line|line.split_once(':').map(|(key,value)|(key.to_ascii_lowercase(),value.trim().to_owned()))).collect::<BTreeMap<_,_>>();
            let length=headers.get("content-length").and_then(|value|value.parse::<usize>().ok()).unwrap_or(0);
            while bytes.len()<end+length {let n=socket.read(&mut part).await.unwrap();if n==0{return}bytes.extend_from_slice(&part[..n]);}
            captured.lock().unwrap().push(Request {target,headers,body:bytes[end..end+length].to_vec()});
            if !reply.delay.is_zero(){tokio::time::sleep(reply.delay).await;}
            if !reply.drop_connection {let headers=reply.headers.iter().map(|(key,value)|format!("{key}: {value}\r\n")).collect::<String>();let output=format!("HTTP/1.1 {} Reply\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{}",reply.status,reply.body.len(),reply.body);let _=socket.write_all(output.as_bytes()).await;}
        }});
        let client=Client::build(&base,&base,Duration::from_millis(300),Duration::from_millis(1),false).unwrap();(client,requests,task)
    }
    fn token()->Token {Token::new("ghu_test-secret".into()).unwrap()}
    fn device()->Value {json!({"device_code":"private-device-code","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","expires_in":900,"interval":1})}
    fn form(request:&Request)->BTreeMap<String,String> {reqwest::Url::parse(&format!("http://localhost/?{}",String::from_utf8_lossy(&request.body))).unwrap().query_pairs().map(|(key,value)|(key.into_owned(),value.into_owned())).collect()}
    #[tokio::test]
    async fn device_flow_polls_pending_and_slow_down_without_scopes_or_client_secrets() {
        let (client,requests,server)=mock(vec![reply(device()),reply(json!({"error":"authorization_pending"})),reply(json!({"error":"slow_down","interval":6})),reply(json!({"access_token":"ghu_access-secret","refresh_token":"ghr_refresh-secret","expires_in":28800,"refresh_token_expires_in":15897600,"token_type":"bearer"}))]).await;
        let session=client.start_auth("Iv1.publicclient").await.unwrap();assert_eq!(session.user_code,"ABCD-EFGH");assert!(!format!("{session:?}").contains("private-device-code"));
        let token=client.poll_auth(&session,&AtomicBool::new(false)).await.unwrap();assert_eq!(token.access_token(),"ghu_access-secret");assert_eq!(token.refresh_token(),Some("ghr_refresh-secret"));assert!(token.expires_at().is_some());
        server.await.unwrap();let requests=requests.lock().unwrap();assert_eq!(requests.len(),4);assert_eq!(requests[0].target,"/login/device/code");assert_eq!(form(&requests[0]).get("client_id").unwrap(),"Iv1.publicclient");
        for request in requests.iter(){let fields=form(request);assert!(!fields.contains_key("scope") && !fields.contains_key("client_secret"));assert!(!request.headers.contains_key("authorization"));}
        assert_eq!(form(&requests[1])["grant_type"],"urn:ietf:params:oauth:grant-type:device_code");assert!(!format!("{token:?}").contains("access-secret"));
    }
    #[tokio::test]
    async fn automatic_publishing_uses_secret_gist_and_fixed_issue_route_with_retry_markers() {
        let (client,requests,server)=mock(vec![created(json!({"id":"abc123","html_url":"https://evil.example/gist"})),created(json!({"number":12,"html_url":"https://evil.example/issue"}))]).await;
        let files=BTreeMap::from([("report.md".into(),"Report details".into())]);let gist=client.upload_gist(&token(),&files,"report-1234").await.unwrap();assert_eq!(gist.url,"https://gist.github.com/abc123");
        let published=client.publish_issue(&token(),"Game failed","Description","report-1234").await.unwrap();assert_eq!(published.url,"https://github.com/usersina/yo3-revival/issues/12");
        server.await.unwrap();let requests=requests.lock().unwrap();assert_eq!(requests[0].target,"/gists");assert_eq!(requests[1].target,"/repos/usersina/yo3-revival/issues");
        let gist:Value=serde_json::from_slice(&requests[0].body).unwrap();assert_eq!(gist["public"],false);assert!(gist["description"].as_str().unwrap().contains("report-1234"));assert!(gist["files"]["README.md"]["content"].as_str().unwrap().contains("<!-- yo3-report:report-1234 -->"));
        let issue:Value=serde_json::from_slice(&requests[1].body).unwrap();assert!(issue["body"].as_str().unwrap().contains("<!-- yo3-report:report-1234 -->"));assert_eq!(issue.as_object().unwrap().len(),2);
        for request in requests.iter(){assert_eq!(request.headers["authorization"],"Bearer ghu_test-secret");}
    }
    #[tokio::test]
    async fn a_dropped_create_response_is_uncertain_and_never_retried() {
        let (client,requests,server)=mock(vec![Reply {drop_connection:true,..reply(Value::Null)}]).await;
        let error=client.publish_issue(&token(),"Title","Body","report-1234").await.unwrap_err();assert_eq!(error.stage,Stage::Publish);assert!(error.uncertain);assert!(!error.message.contains("ghu_test-secret"));
        server.await.unwrap();assert_eq!(requests.lock().unwrap().len(),1);
    }
    #[tokio::test]
    async fn refresh_rotates_access_and_refresh_tokens_without_a_client_secret() {
        let old=Token::from_secrets("ghu_old-access".into(),Some(1),Some("ghr_old-refresh".into()),Some(now()+3600)).unwrap();
        let (client,requests,server)=mock(vec![reply(json!({"access_token":"ghu_new-access","refresh_token":"ghr_new-refresh","expires_in":28800,"refresh_token_expires_in":15897600,"token_type":"bearer"}))]).await;
        let new=client.refresh_token("Iv1.publicclient",&old).await.unwrap();server.await.unwrap();
        assert_eq!(new.access_token(),"ghu_new-access");assert_eq!(new.refresh_token(),Some("ghr_new-refresh"));assert!(new.expires_at().unwrap()>now());assert!(new.refresh_expires_at().unwrap()>now());
        let requests=requests.lock().unwrap();let fields=form(&requests[0]);assert_eq!(requests[0].target,"/login/oauth/access_token");assert_eq!(fields["grant_type"],"refresh_token");assert_eq!(fields["refresh_token"],"ghr_old-refresh");assert!(!fields.contains_key("client_secret") && !fields.contains_key("scope"));assert!(!requests[0].headers.contains_key("authorization"));
    }
    #[tokio::test]
    async fn device_authorization_decline_and_both_expiry_codes_stop_polling_safely() {
        for code in ["access_denied","expired_token","token_expired"] {
            let (client,requests,server)=mock(vec![reply(device()),reply(json!({"error":code,"error_description":"private-device-code ghu_test-secret /home/private"}))]).await;
            let session=client.start_auth("Iv1.publicclient").await.unwrap();let error=client.poll_auth(&session,&AtomicBool::new(false)).await.unwrap_err();server.await.unwrap();
            assert_eq!(error.stage,Stage::Poll);assert_eq!(error.auth_expired,code!="access_denied");assert!(!error.message.contains("private-device-code") && !error.message.contains("ghu_test-secret") && !error.message.contains("/home/private"));assert_eq!(requests.lock().unwrap().len(),2);
        }
    }
    #[tokio::test]
    async fn cancellation_and_local_expiry_never_send_a_poll_request() {
        let (client,requests,server)=mock(vec![reply(device())]).await;let mut session=client.start_auth("Iv1.publicclient").await.unwrap();server.await.unwrap();
        let error=client.poll_auth(&session,&AtomicBool::new(true)).await.unwrap_err();assert!(error.message.contains("cancelled"));
        session.started=Instant::now()-Duration::from_secs(2);let error=client.poll_auth(&session,&AtomicBool::new(false)).await.unwrap_err();assert!(error.auth_expired);assert_eq!(requests.lock().unwrap().len(),1);
    }
    #[tokio::test]
    async fn verification_url_is_fixed_and_never_controlled_by_a_response() {
        let mut response=device();response["verification_uri"]=json!("https://evil.example/steal");let (client,requests,server)=mock(vec![reply(response)]).await;
        let error=client.start_auth("Iv1.publicclient").await.unwrap_err();server.await.unwrap();assert_eq!(error.stage,Stage::Authorize);assert!(!error.message.contains("evil.example"));assert_eq!(requests.lock().unwrap().len(),1);
    }
    #[tokio::test]
    async fn redirects_are_not_followed_and_credentials_are_not_forwarded() {
        let redirect=Reply {status:302,headers:vec![("Location".into(),"https://evil.example/steal".into())],..reply(json!({}))};
        let (client,requests,server)=mock(vec![redirect]).await;let error=client.identity(&token()).await.unwrap_err();server.await.unwrap();
        assert_eq!(error.stage,Stage::Identity);assert!(error.message.contains("No credentials were forwarded"));assert_eq!(requests.lock().unwrap().len(),1);
    }
    #[tokio::test]
    async fn revoked_token_and_permission_failures_have_typed_safe_recovery() {
        for status in [401,403,429] {
            let (client,requests,server)=mock(vec![Reply {status,..reply(json!({"message":"ghu_test-secret raw private reply"}))}]).await;
            let error=client.publish_issue(&token(),"Title","Body","report-1234").await.unwrap_err();server.await.unwrap();assert_eq!(error.auth_expired,status==401);assert!(!error.uncertain);assert!(!error.message.contains("ghu_test-secret") && !error.message.contains("raw private reply"));assert_eq!(requests.lock().unwrap().len(),1);
        }
    }
    #[tokio::test]
    async fn malformed_or_oversized_success_after_create_remains_uncertain() {
        for body in ["not-json".into()," ".repeat(MAX_RESPONSE+1)] {
            let (client,requests,server)=mock(vec![Reply {body,..created(Value::Null)}]).await;
            let error=client.publish_issue(&token(),"Title","Body","report-1234").await.unwrap_err();server.await.unwrap();assert!(error.uncertain);assert_eq!(error.stage,Stage::Publish);assert_eq!(requests.lock().unwrap().len(),1);
        }
    }
    #[tokio::test]
    async fn gist_and_issue_recovery_require_the_exact_marker_and_current_user() {
        let marker="<!-- yo3-report:report-1234 -->";
        let (client,requests,server)=mock(vec![
            reply(json!([{"id":"dead","description":"YO3 diagnostics <!-- yo3-report:report-12345 -->"},{"id":"abc123","description":format!("YO3 diagnostics {marker}")} ])),
            reply(json!({"files":{"README.md":{"content":format!("Diagnostic report\n{marker}\n")}}})),
            reply(json!({"login":"player-one","html_url":"https://evil.example"})),
            reply(json!([
                {"number":1,"body":marker,"user":{"login":"player-one"},"pull_request":{}},
                {"number":2,"body":marker,"user":{"login":"another-player"}},
                {"number":3,"body":format!("extra{marker}extra"),"user":{"login":"player-one"}},
                {"number":4,"body":format!("Report\n\n{marker}"),"user":{"login":"player-one"},"html_url":"https://evil.example"}
            ]))
        ]).await;
        assert_eq!(client.find_gist(&token(),"report-1234").await.unwrap().unwrap().url,"https://gist.github.com/abc123");assert_eq!(client.find_issue(&token(),"report-1234").await.unwrap().unwrap().number,4);server.await.unwrap();
        let requests=requests.lock().unwrap();assert_eq!(requests[0].target,"/gists?per_page=30&page=1");assert_eq!(requests[1].target,"/gists/abc123");assert_eq!(requests[2].target,"/user");assert!(requests[3].target.starts_with("/repos/usersina/yo3-revival/issues?creator=player-one&state=all"));
    }
    #[tokio::test]
    async fn local_request_limits_and_invalid_ids_fail_before_any_network_request() {
        let (client,requests,server)=mock(vec![]).await;server.await.unwrap();
        let files=BTreeMap::from([("report.md".into(),"x".repeat(MAX_BODY))]);let error=client.upload_gist(&token(),&files,"report-1234").await.unwrap_err();assert!(error.message.contains("8 MiB"));assert!(!error.uncertain);
        let files=BTreeMap::from([("../private.key".into(),"secret".into())]);assert!(client.upload_gist(&token(),&files,"report-1234").await.is_err());assert!(client.publish_issue(&token(),"Title","Body","bad/path").await.is_err());assert!(client.start_auth("client-secret\nheader").await.is_err());assert_eq!(requests.lock().unwrap().len(),0);
    }
    #[tokio::test]
    async fn old_saved_tokens_expire_without_sending_credentials() {
        let (client,requests,server)=mock(vec![]).await;server.await.unwrap();let expired=Token::from_secret("ghu_expired".into(),Some(now()-1)).unwrap();
        assert!(client.identity(&expired).await.unwrap_err().auth_expired);assert!(client.refresh_token("Iv1.publicclient",&expired).await.unwrap_err().auth_expired);assert_eq!(requests.lock().unwrap().len(),0);
    }

    #[tokio::test]
    async fn cancellation_interrupts_an_in_flight_device_poll() {
        let (client,requests,server)=mock(vec![reply(device()),Reply {delay:Duration::from_secs(1),..reply(json!({"error":"authorization_pending"}))}]).await;
        let session=client.start_auth("Iv1.publicclient").await.unwrap();let cancel=Arc::new(AtomicBool::new(false));let worker_cancel=cancel.clone();
        let worker=tokio::spawn(async move {client.poll_auth(&session,&worker_cancel).await});
        tokio::time::timeout(Duration::from_millis(200),async {while requests.lock().unwrap().len()<2 {tokio::time::sleep(Duration::from_millis(1)).await;}}).await.unwrap();
        cancel.store(true,Ordering::Release);let result=tokio::time::timeout(Duration::from_millis(100),worker).await.unwrap().unwrap().unwrap_err();assert!(result.message.contains("cancelled"));assert_eq!(requests.lock().unwrap().len(),2);server.abort();
    }
    #[tokio::test]
    async fn an_oauth_http_failure_cannot_be_accepted_as_a_successful_token() {
        let (client,requests,server)=mock(vec![Reply {status:400,..reply(json!({"access_token":"ghu_false-success","refresh_token":"ghr_false-success","token_type":"bearer"}))}]).await;
        let old=Token::from_secrets("ghu_old-access".into(),None,Some("ghr_old-refresh".into()),None).unwrap();let error=client.refresh_token("Iv1.publicclient",&old).await.unwrap_err();server.await.unwrap();assert_eq!(error.stage,Stage::Refresh);assert!(!error.message.contains("false-success"));assert_eq!(requests.lock().unwrap().len(),1);
    }
    #[tokio::test]
    async fn reconciliation_stops_at_a_bounded_page_budget_without_claiming_absence() {
        let page=Value::Array(vec![json!({"description":"unrelated gist"});PAGE_SIZE]);let replies=(0..MAX_PAGES).map(|_|reply(page.clone())).collect();
        let (client,requests,server)=mock(replies).await;let error=client.find_gist(&token(),"report-1234").await.unwrap_err();server.await.unwrap();assert_eq!(error.stage,Stage::FindGist);assert!(error.message.contains("could not be ruled out"));assert_eq!(requests.lock().unwrap().len(),MAX_PAGES);
    }

    #[tokio::test]
    async fn the_generated_marker_file_is_included_in_the_gist_file_limit() {
        let (client,requests,server)=mock(vec![]).await;server.await.unwrap();let files=(0..100).map(|index|(format!("session-{index}.log"),"entry".into())).collect();
        let error=client.upload_gist(&token(),&files,"report-1234").await.unwrap_err();assert!(!error.uncertain);assert!(error.message.contains("file list"));assert_eq!(requests.lock().unwrap().len(),0);
    }

}
