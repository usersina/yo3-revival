//! Real dashboard responses: source artwork, bounded reads, authorization and HTTP revalidation.
use std::{sync::{Arc,Mutex},path::PathBuf};
use axum::{body::{Body,to_bytes},extract::ConnectInfo,http::{Request,StatusCode},Router};
use tower::ServiceExt;
use yo3_server::{accounts::Directory,api::Api,dashboard::{self,Dashboard},gifts::{Card,Gifts},logins::{self,Login,Registration,Role}};
struct Fixture { dir: PathBuf, router: Router }
fn archive(path: &std::path::Path, entries: &[(&str, Vec<u8>)]) {
    let mut bytes=Vec::new();
    for (name,payload) in entries {
        let mut h=[0;30]; h[..4].copy_from_slice(b"YO\x03\x04");
        h[18..22].copy_from_slice(&(payload.len() as u32).to_le_bytes()); h[22..26].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        h[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes()); bytes.extend(h); bytes.extend(name.as_bytes()); bytes.extend(payload);
    }
    std::fs::write(path,bytes).unwrap();
}
fn png(w:u32,h:u32)->Vec<u8> { [b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".as_slice(),&w.to_be_bytes(),&h.to_be_bytes(),&[8,6,0,0,0]].concat() }
impl Fixture {
    async fn new(name:&str, images:bool)->Self {
        logins::use_fast_hashing();
        let dir=std::env::temp_dir().join(format!("yo3-card-preview-{name}-{}",std::process::id())); let _=std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("package")).unwrap();
        if images { archive(&dir.join("package/data000.ypk"), &[("data/mini/4007_2.png",png(50,72))]);
            archive(&dir.join("package/data001.ypk"), &[("data/card/4007_2.png",png(200,290)),("data/card/4007_1.png",vec![0;600_000])]); }
        let mut accounts=Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let password=logins::hash_password("PreviewPassword1".into()).await.unwrap();
        for (name,role) in [("PreviewAdmin",Role::Admin),("PreviewMod",Role::Moderator)] {
            accounts.register(Login::new(name,password.clone(),logins::digest(&logins::new_code())),None,Registration::Open).unwrap(); accounts.set_role(name,role).unwrap();
        }
        let mut api=Api::new(Arc::new(Mutex::new(accounts)),Arc::new(yo3_server::settings::Live::open(None,vec![]).unwrap()),None,dir.clone(),"test")
            .with_gifts(Gifts { cards: vec![Card { row:817,cid:4007,art:2,name:"Example dragon".into(),description:"Original effect text.\nSecond line.".into() },Card { row:816,cid:4007,art:1,name:"Other artwork".into(),description:String::new() }],recipes:vec![] });
        api.titles=Some(Arc::new(Default::default()));
        let router=dashboard::router(Dashboard { api:Arc::new(api), panel:Arc::new(yo3_server::panel::Panel::with_policy(yo3_server::staff_mfa::StaffMfaPolicy::Optional)) });
        Self { dir,router }
    }
    async fn request(&self,path:&str,cookie:&str,etag:Option<&str>)->axum::response::Response {
        let mut r=Request::builder().uri(path).header("host","localhost").header("cookie",cookie);
        if let Some(etag)=etag { r=r.header("if-none-match",etag); }
        let mut r=r.body(Body::empty()).unwrap(); r.extensions_mut().insert(ConnectInfo("127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap()));
        self.router.clone().oneshot(r).await.unwrap()
    }
    async fn login(&self,name:&str)->String {
        let mut r=Request::builder().method("POST").uri("/panel/api/v1/sign-in").header("host","localhost").header("origin","http://localhost")
            .header("idempotency-key","card-preview-signin-0001").header("content-type","application/json")
            .body(Body::from(serde_json::json!({"user":name,"password":"PreviewPassword1"}).to_string())).unwrap();
        r.extensions_mut().insert(ConnectInfo("127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap()));
        let res=self.router.clone().oneshot(r).await.unwrap(); assert_eq!(res.status(),StatusCode::OK);
        res.headers()["set-cookie"].to_str().unwrap().split(';').next().unwrap().into()
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _=std::fs::remove_dir_all(&self.dir); } }
#[tokio::test]
async fn card_images_are_private_versioned_and_revalidated_only_after_authentication() {
    let f=Fixture::new("cache",true).await; let cookie=f.login("PreviewAdmin").await;
    let res=f.request("/panel/api/v1/gifts/choices",&cookie,None).await;
    assert_eq!(res.headers()["cache-control"],"no-store");
    let data:serde_json::Value=serde_json::from_slice(&to_bytes(res.into_body(),1_000_000).await.unwrap()).unwrap();
    let card=&data["cards"][0]; assert_eq!(card["description"],"Original effect text.\nSecond line.");
    let url=card["image_url"].as_str().unwrap(); assert!(url.starts_with("/panel/api/v1/cards/817/full?v="));
    let res=f.request(url,&cookie,None).await; assert_eq!(res.status(),200); assert_eq!(res.headers()["content-type"],"image/png");
    assert_eq!(res.headers()["cache-control"],"private, max-age=86400"); let etag=res.headers()["etag"].to_str().unwrap().to_string();
    assert_eq!(to_bytes(res.into_body(),1_000_000).await.unwrap(),png(200,290));
    let res=f.request(url,&cookie,Some(&format!("\"another\", W/{etag}"))).await; assert_eq!(res.status(),304); assert!(to_bytes(res.into_body(),1_000).await.unwrap().is_empty());
    for (cookie,status) in [(String::new(),401),(f.login("PreviewMod").await,403)] {
        let res=f.request(url,&cookie,Some(&etag)).await; assert_eq!(res.status(),status); assert_eq!(res.headers()["cache-control"],"no-store");
    }
    for path in [url.replace("817/full","999/full"),url.replace("817/full","817/wrong"),url.replace("?v=","?v=stale"),"/panel/api/v1/cards/817/full".into(),url.replace("817/full","816/full")] {
        let res=f.request(&path,&cookie,None).await; assert!(res.status().is_client_error()); assert_eq!(res.headers()["cache-control"],"no-store");
    }
    let thumb=card["thumbnail_url"].as_str().unwrap(); let res=f.request(thumb,&cookie,None).await; assert_eq!(res.status(),200);
    assert_eq!(to_bytes(res.into_body(),1_000_000).await.unwrap(),png(50,72));
}
#[tokio::test]
async fn missing_image_archives_leave_the_catalogue_and_text_available() {
    let f=Fixture::new("absent",false).await; let cookie=f.login("PreviewAdmin").await;
    let res=f.request("/panel/api/v1/gifts/choices",&cookie,None).await; assert_eq!(res.status(),200);
    let data:serde_json::Value=serde_json::from_slice(&to_bytes(res.into_body(),1_000_000).await.unwrap()).unwrap();
    assert!(data["cards"][0]["thumbnail_url"].is_null()); assert!(data["cards"][0]["image_url"].is_null());
    assert_eq!(data["cards"][0]["description"],"Original effect text.\nSecond line.");
}

#[test]
fn installed_game_images_match_original_artwork_and_descriptions_when_available() {
    let dir=yo3_data::default_game_dir(); if !dir.join("package/data001.ypk").exists() { return }
    let data=yo3_data::GameData::load(&dir).unwrap();
    let gifts=Gifts::from_game(&data);
    let card=gifts.cards.iter().find(|c|c.cid==4007 && c.art==2).unwrap(); let row=card.row;
    assert!(!card.description.is_empty());
    let api=Api::new(Arc::new(Mutex::new(Directory::new(vec![]))),Arc::new(yo3_server::settings::Live::open(None,vec![]).unwrap()),None,dir,"test").with_gifts(gifts);
    let mut api=api; api.titles=Some(Arc::new(Default::default()));
    for (filename,folder,size,width) in [("data000.ypk","mini",yo3_server::api::card_images::CardImageSize::Mini,50u32),("data001.ypk","card",yo3_server::api::card_images::CardImageSize::Full,200)] {
        // The image revision is opaque: catalogue supplies the server-generated URL, never the test.
        let choices=api.gift_choices("4007",yo3_server::api::Actor::Host { by:"test" }).unwrap();
        let card=choices.cards.iter().find(|c|c.row==row).unwrap();
        let url=if width==50 {card.thumbnail_url.as_ref()}else{card.image_url.as_ref()}.unwrap();
        let revision=url.split_once("?v=").unwrap().1;
        let image=api.card_image(row,size,revision,yo3_server::api::Actor::Host {by:"test"}).unwrap();
        let original=yo3_data::Ypk::open(api.game_dir.join("package").join(filename)).unwrap().read(&format!("data/{folder}/4007_2.png")).unwrap().unwrap();
        assert_eq!(image.bytes,original); assert_eq!(u32::from_be_bytes(image.bytes[16..20].try_into().unwrap()),width);
    }
}

#[tokio::test]
async fn non_png_and_wrong_dimensions_are_never_served_as_a_cacheable_success() {
    for (name,bytes) in [("wrong-size",png(200,289)),("not-png",b"<html>changed archive</html>".to_vec())] {
        let f=Fixture::new(name,true).await;
        archive(&f.dir.join("package/data001.ypk"),&[("data/card/4007_2.png",bytes)]);
        let cookie=f.login("PreviewAdmin").await;
        let res=f.request("/panel/api/v1/gifts/choices",&cookie,None).await;
        let data:serde_json::Value=serde_json::from_slice(&to_bytes(res.into_body(),1_000_000).await.unwrap()).unwrap();
        let url=data["cards"][0]["image_url"].as_str().unwrap();
        let res=f.request(url,&cookie,None).await; assert_eq!(res.status(),404); assert_eq!(res.headers()["cache-control"],"no-store");
    }
}
