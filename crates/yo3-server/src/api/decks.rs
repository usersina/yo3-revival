//! EDOPro decks for a player (`crate::ydk`) through the admin API: `decks` lists the player's decks, `export-deck`
//! writes one as a `.ydk` file and a `ydke://` link, `import-deck` keeps a `.ydk` file or link as one of the player's
//! uploaded decks, so it is in Load Deck's list the next time the game asks for it. A player is named, else the first
//! player of the server (your own server in Desktop). Players on a friend's server use TLS operation 8 instead.
use serde_json::{Value, json};
use crate::{lock::Locked, logins::Role, store::{self, AuditRow, Receipt}, ydk::{self, Passcodes}};
use super::{Api, Actor, Op, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};

fn invalid(e: impl ToString) -> MutationError { MutationError::new(MutationErrorCode::InvalidRequest, e.to_string()) }
fn missing() -> MutationError { MutationError::new(MutationErrorCode::NotFound, "No such player.") }
fn unavailable(e: impl std::fmt::Display) -> MutationError { MutationError::new(MutationErrorCode::PersistenceFailed, format!("The card data is unavailable: {e}")) }

/// The passcodes of the rules engine's data (`crate::engine::Engine::dir`).
pub fn passcodes() -> Result<Passcodes, String> {
    let dir = crate::engine::Engine::dir().ok_or("the rules engine's folder is missing")?;
    Passcodes::load(&dir)
}

impl Api {
    /// The named player, else the first: (player id, name).
    pub(super) fn deck_player(d: &crate::accounts::Directory, player: Option<&str>) -> Option<(u32, String)> {
        let id = match player {
            Some(name) => d.player_id_named(name)?,
            None => d.entries().flat_map(|(_, _, a)| a.players.iter().map(|p| p.player_id)).min()?,
        };
        d.player(id).map(|p| (id, p.name.clone()))
    }

    fn starter(&self) -> Option<crate::front::Deck> {
        self.world.as_ref().and_then(|w| w.starter_deck.clone())
    }

    /// `decks`: each deck's name and its main, Extra and side deck sizes.
    pub(super) fn list_decks(&self, player: Option<&str>) -> Result<Value, String> {
        let d = self.accounts.locked();
        let (id, name) = Self::deck_player(&d, player).ok_or("No such player.")?;
        let p = d.player(id).ok_or("No such player.")?;
        Ok(ydk::list(&name, &ydk::decks_of(p, self.starter().as_ref())))
    }

    /// `export-deck`: the deck as a `.ydk` file and a `ydke://` link.
    pub(super) fn export_deck(&self, player: Option<&str>, deck: &str) -> Result<Value, String> {
        let gifts = self.gifts()?;
        let passcodes = passcodes()?;
        let starting = self.starting_rows();
        let d = self.accounts.locked();
        let (id, name) = Self::deck_player(&d, player).ok_or("No such player.")?;
        let p = d.player(id).ok_or("No such player.")?;
        let cards = ydk::Cards { cards: &gifts.cards, starting: &starting, passcodes: &passcodes };
        ydk::export_named(&name, &ydk::decks_of(p, self.starter().as_ref()), deck, &cards)
    }

    /// `import-deck`: the `.ydk` file or `ydke://` link `text` kept as the player's deck `name` (`ydk::import_for`), in one
    /// commit with the audit row and the receipt.
    pub(super) fn import_deck(&self, player: Option<&str>, name: &str, text: &str, id: &str, actor: Actor<'_>, origin: &str) -> Result<Value, MutationError> {
        validate_id(id)?;
        let op = Op::ImportDeck { player: player.map(str::to_owned), name: name.to_owned(), ydk: text.to_owned() };
        let gifts = self.gifts().map_err(unavailable)?;
        let passcodes = passcodes().map_err(unavailable)?;
        let starting = self.starting_rows();
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, Role::Admin)?;
        let operation = op.name(); let hash = fingerprint(&op)?;
        if let Some(result) = self.replay::<Value>(&identity.scope, id, &operation, &hash)? { return Ok(result) }
        let (player_id, player_name) = Self::deck_player(&d, player).ok_or_else(missing)?;
        let p = d.player(player_id).ok_or_else(missing)?;
        let cards = ydk::Cards { cards: &gifts.cards, starting: &starting, passcodes: &passcodes };
        let kept = ydk::import_for(p, &ydk::decks_of(p, self.starter().as_ref()), name, text, &cards).map_err(invalid)?;
        let mut next = d.prepare_player(player_id, |p| { p.uploaded.push(kept.deck.clone()); Ok(()) }).map_err(invalid)?;
        let result = kept.json(&player_name);
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by, origin: origin.into(), op: operation.clone(),
            target: op.target(), before: None, after: Some(json!({ "deck": kept.deck.name, "cards": kept.deck.cards().count() })), detail: Some(json!({ "request_id": id })) };
        let receipt = Receipt { operation, input: hash, result: result.clone() };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        Ok(result)
    }
}
