//! Original game artwork, never a caller-supplied filename or an external image host.
use std::{path::Path, time::UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use yo3_data::Ypk;
use super::{Api, Actor, MutationError, MutationErrorCode};
use crate::{lock::Locked, logins::Role};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all="lowercase")]
pub enum CardImageSize { Mini, Full }
impl CardImageSize {
    fn directory(self)-> &'static str { match self { Self::Mini=>"mini",Self::Full=>"card" } }
    fn segment(self)-> &'static str { match self { Self::Mini=>"mini",Self::Full=>"full" } }
    fn dimensions(self)->(u32,u32) { match self { Self::Mini=>(50,72),Self::Full=>(200,290) } }
}
struct Source { archive: Ypk, revision: String, available: std::collections::HashSet<String> }
impl Source {
    fn open(path:&Path)->Option<Self> {
        if !path.exists() { return None }
        let result=(|| -> Result<Self,Box<dyn std::error::Error>> {
            let meta=std::fs::metadata(path)?;
            let stamp=format!("{}:{}",meta.len(),meta.modified()?.duration_since(UNIX_EPOCH)?.as_nanos());
            let archive=Ypk::open(path)?;
            let available=archive.entries().iter().filter(|e|e.size<=512*1024).map(|e|e.name.clone()).collect();
            Ok(Self { archive,revision:hash(stamp.as_bytes()),available })
        })();
        match result { Ok(source)=>Some(source),Err(e)=> { tracing::warn!("Card image archive {} unavailable: {e}",path.display()); None } }
    }
}
#[derive(Default)]
pub(super) struct CardImages { mini:Option<Source>,full:Option<Source> }
impl CardImages {
    fn open(dir:&Path)->Self { Self { mini:Source::open(&dir.join("package/data000.ypk")),full:Source::open(&dir.join("package/data001.ypk")) } }
    fn source(&self,size:CardImageSize)->Option<&Source> { match size { CardImageSize::Mini=>self.mini.as_ref(),CardImageSize::Full=>self.full.as_ref() } }
}
pub struct CardImage { pub bytes:Vec<u8>,pub etag:String }
fn hash(bytes:&[u8])->String { ring::digest::digest(&ring::digest::SHA256,bytes).as_ref().iter().map(|b|format!("{b:02x}")).collect() }
fn missing()->MutationError { MutationError::new(MutationErrorCode::NotFound,"This card image is unavailable. Ask the host to check the original image archives.") }
impl Api {
    fn card_images(&self)->&CardImages { self.card_images.get_or_init(||CardImages::open(&self.game_dir)) }
    pub(super) fn card_image_url(&self,card:&crate::gifts::Card,size:CardImageSize)->Option<String> {
        let source=self.card_images().source(size)?;
        let name=format!("data/{}/{}_{}.png",size.directory(),card.cid,card.art);
        source.available.contains(&name).then(||format!("/panel/api/v1/cards/{}/{}?v={}",card.row,size.segment(),source.revision))
    }
    pub fn card_image(&self,row:u16,size:CardImageSize,revision:&str,actor:Actor<'_>)->Result<CardImage,MutationError> {
        actor.resolve(&self.accounts.locked(),Role::Admin)?;
        let gifts=self.gifts().map_err(|_|missing())?;
        let card=gifts.cards.iter().find(|c|c.row==row).ok_or_else(missing)?;
        let source=self.card_images().source(size).filter(|s|s.revision==revision).ok_or_else(missing)?;
        let name=format!("data/{}/{}_{}.png",size.directory(),card.cid,card.art);
        let bytes=source.archive.read_bounded(&name,512*1024).map_err(|e| { tracing::warn!("Card image {name} could not be read: {e}"); missing() })?.ok_or_else(missing)?;
        // A fixed PNG type and original dimensions only. Never serve HTML/SVG from a changed archive.
        if bytes.len()<29 || &bytes[..16]!=b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR" { return Err(missing()) }
        let dimensions=(u32::from_be_bytes(bytes[16..20].try_into().unwrap()),u32::from_be_bytes(bytes[20..24].try_into().unwrap()));
        if dimensions!=size.dimensions() { return Err(missing()) }
        let etag=format!("\"{}\"",hash(&bytes));
        Ok(CardImage { bytes,etag })
    }
}
