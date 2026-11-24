//! Transactional gifts: player, mail, audit and receipt first; live collection/present notifications afterwards.
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use crate::{lock::Locked, logins::Role, mail::Mailbox, shop::Present, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, Op, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GiftRequest {
    Cards { #[schema(maximum = 4294967295_u32)] player_id: u32, row: u16, #[schema(minimum = 1, maximum = 255)] count: u32 },
    Recipe { #[schema(maximum = 4294967295_u32)] player_id: u32, #[schema(maximum = 4294967295_u32)] deck_id: u32, include_cards: bool },
    Title { #[schema(maximum = 4294967295_u32)] player_id: u32, title_id: u16 },
}
impl GiftRequest {
    fn player(&self) -> u32 { match self { Self::Cards { player_id, .. } | Self::Recipe { player_id, .. } | Self::Title { player_id, .. } => *player_id } }
    fn operation(&self) -> &'static str { match self { Self::Cards { .. } => "give-cards", Self::Recipe { .. } => "give-recipe", Self::Title { .. } => "grant-title" } }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GiftOutcome {
    Cards { row: u16, #[schema(maximum = 255)] added: u32, #[schema(maximum = 255)] total: u32 },
    Recipe { #[schema(maximum = 4294967295_u32)] deck_id: u32, include_cards: bool },
    Title { title_id: u16, already_owned: bool },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GiftResult {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, #[schema(maximum = 4294967295_u32)] pub player_id: u32,
    pub gift: GiftOutcome, pub request_id: String, pub message: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CardChoice { pub row: u16, pub konami_id: u16, #[schema(maximum = 4294967295_u32)] pub art: u32, pub name: String, pub description: String, pub thumbnail_url: Option<String>, pub image_url: Option<String> }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecipeChoice { #[schema(maximum = 4294967295_u32)] pub deck_id: u32, pub name: String, #[schema(maximum = 4294967295_u32)] pub card_count: u32 }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TitleChoice { pub title_id: u16, pub name: String }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GiftChoices {
    pub cards: Vec<CardChoice>, pub recipes: Vec<RecipeChoice>, pub titles: Vec<TitleChoice>,
    pub more_cards: bool, pub more_recipes: bool, pub more_titles: bool,
}
#[derive(Serialize)]
enum Input { Numeric(GiftRequest), Named(Op) }
impl Input { fn operation(&self) -> String { match self { Self::Numeric(r) => r.operation().into(), Self::Named(op) => op.name() } } }
fn invalid(e: impl ToString) -> MutationError { MutationError::new(MutationErrorCode::InvalidRequest, e.to_string()) }
fn missing() -> MutationError { MutationError::new(MutationErrorCode::NotFound, "The player no longer exists.") }
fn unavailable(e: impl std::fmt::Display) -> MutationError {
    tracing::warn!("Gift catalogue unavailable: {e}");
    MutationError::new(MutationErrorCode::PersistenceFailed, "Gift choices are unavailable. Ask the host to check the game data files.")
}
impl Api {
    pub fn gift_choices(&self, query: &str, actor: Actor<'_>) -> Result<GiftChoices, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Admin)?;
        if query.chars().count() > 80 { return Err(invalid("Search with at most 80 characters.")) }
        let g = self.gifts().map_err(unavailable)?; let t = self.titles().map_err(unavailable)?;
        let q = query.trim().to_lowercase();
        let mut cards: Vec<_> = g.cards.iter().filter(|c| c.name.to_lowercase().contains(&q) || c.cid.to_string() == q || c.row.to_string() == q)
            .collect();
        cards.sort_by(|a,b| (&a.name, a.art, a.row).cmp(&(&b.name, b.art, b.row)));
        let mut recipes: Vec<_> = g.recipes.iter().filter(|r| r.title.to_lowercase().contains(&q) || r.id.to_string() == q)
            .map(|r| RecipeChoice { deck_id: r.id, name: r.title.clone(), card_count: r.cards.len() as u32 }).collect();
        recipes.sort_by(|a,b| (&a.name, a.deck_id).cmp(&(&b.name, b.deck_id)));
        let mut titles: Vec<_> = t.by_name.iter().filter(|(name,id)| name.to_lowercase().contains(&q) || id.to_string() == q)
            .map(|(name, &title_id)| TitleChoice { name: name.clone(), title_id }).collect();
        titles.sort_by(|a,b| (&a.name, a.title_id).cmp(&(&b.name, b.title_id)));
        let (more_cards, more_recipes, more_titles) = (cards.len() > 50, recipes.len() > 50, titles.len() > 50);
        cards.truncate(50); recipes.truncate(50); titles.truncate(50);
        let cards = cards.into_iter().map(|c| CardChoice { row: c.row, konami_id: c.cid, art: c.art, name: c.name.clone(), description: c.description.clone(), thumbnail_url: self.card_image_url(c, super::card_images::CardImageSize::Mini), image_url: self.card_image_url(c, super::card_images::CardImageSize::Full) }).collect();
        Ok(GiftChoices { cards, recipes, titles, more_cards, more_recipes, more_titles })
    }
    pub fn give_gift(&self, request: &GiftRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<GiftResult, MutationError> {
        self.gift_change(Input::Numeric(request.clone()), id, actor, origin)
    }
    pub(super) fn named_gift(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<Value, MutationError> {
        let result = self.gift_change(Input::Named(op.clone()), id, actor, origin)?;
        let mut value = json!(result);
        // Rebuild human-facing labels from current metadata, never from permanent receipts. If files are gone,
        // the committed generic confirmation is still usable; presentation cannot turn success into failure.
        let message = match (&result.gift, op) {
            (GiftOutcome::Cards { row, added, total }, Op::GiveCards { player, .. }) => {
                value["row"] = json!(row); value["count"] = json!(total);
                // An anime-only card (#75) while anime cards are off: given, but no duel takes a deck with it until they're on.
                let anime_off = self.world.as_ref().is_some_and(|w| !w.settings.now().anime_cards
                    && w.added.cards.iter().any(|c| c.row == *row && c.anime_only));
                let warning = if anime_off { ". It is an anime card: anime cards are off, so no duel takes a deck with it until a host turns them on" } else { "" };
                self.gifts().ok().and_then(|g| g.cards.iter().find(|c| c.row == *row).map(|c|
                    format!("gave {added} × {} (artwork {}, Konami id {}) to {player}: {total} beyond their starting cards now (shown at the next map visit){warning}", c.name, c.art, c.cid)))
            }
            (GiftOutcome::Recipe { deck_id, include_cards }, Op::GiveRecipe { player, .. }) => self.gifts().ok().and_then(|g| g.recipes.iter().find(|r| r.id == *deck_id).map(|r| {
                let with = if *include_cards { format!(" and its {} cards",r.cards.len()) } else { String::new() };
                format!("gave the recipe {} (deck {deck_id}){with} to {player} (on Load Deck's Official tab after their next login or duel)", r.title)
            })),
            (GiftOutcome::Title { title_id, .. }, Op::GrantTitle { player, .. }) => self.titles().ok().and_then(|t| t.name(*title_id).map(|name| format!("{player} has the title {name} (they wear it in Avatar Settings)"))),
            _ => None,
        };
        if let Some(message) = message { value["message"] = json!(message); }
        Ok(value)
    }
    fn gift_change(&self, input: Input, id: &str, actor: Actor<'_>, origin: &str) -> Result<GiftResult, MutationError> {
        validate_id(id)?;
        let mut hub = self.hub.as_ref().map(|h| h.locked());
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, Role::Admin)?;
        let operation = input.operation(); let hash = fingerprint(&input)?;
        if let Some(result) = self.replay(&identity.scope, id, &operation, &hash)? { return Ok(result) }
        let request = match &input {
            Input::Numeric(r) => r.clone(),
            Input::Named(op) => {
                let name = op.account().ok_or_else(missing)?;
                let player_id = d.player_id_named(name).ok_or_else(missing)?;
                match op {
                    Op::GiveCards { card, art, count, .. } => GiftRequest::Cards { player_id, row: self.gifts().map_err(unavailable)?.card(card, *art).map_err(invalid)?.row, count: count.unwrap_or(1) },
                    Op::GiveRecipe { deck, cards, .. } => GiftRequest::Recipe { player_id, deck_id: self.gifts().map_err(unavailable)?.recipe(deck).map_err(invalid)?.id, include_cards: *cards },
                    Op::GrantTitle { title, .. } => GiftRequest::Title { player_id, title_id: crate::titles::by_name(self.titles().map_err(unavailable)?.as_ref(), title).ok_or_else(|| invalid("No such title; titles lists them."))? },
                    _ => unreachable!(),
                }
            }
        };
        let player_id = request.player(); let p = d.player(player_id).ok_or_else(missing)?;
        let user_id = d.entries().find(|(_,_,a)| a.players.iter().any(|p| p.player_id == player_id)).map(|(_,_,a)| a.user_id).ok_or_else(missing)?;
        let mut additions: BTreeMap<u16,u32> = BTreeMap::new();
        let mut item = None; let mut letter = None; let mut present = None;
        let (outcome, before, after) = match request {
            GiftRequest::Cards { row, count, .. } => {
                if !(1..=255).contains(&count) { return Err(invalid("Give 1 to 255 copies.")) }
                let g = self.gifts().map_err(unavailable)?;
                let c = g.cards.iter().find(|c| c.row == row).ok_or_else(|| invalid("Choose a card artwork from the catalogue."))?;
                let before = p.cards.get(&row).copied().unwrap_or_default();
                let total = before.checked_add(count).filter(|n| *n <= 255).ok_or_else(|| invalid("This gift exceeds the additional-card limit (255 per artwork)."))?;
                additions.insert(row, count);
                let art = if g.cards.iter().filter(|v| v.cid == c.cid).count() > 1 { format!(" (artwork {})", c.art) } else { String::new() };
                letter = Some(("Cards received", format!("The host gave you {count} × {}{art}. They are in your cards now; you will see them when you next come back to the map.", c.name)));
                present = Some(Present::Gift { cards: vec![row; count.min(5) as usize] });
                (GiftOutcome::Cards { row, added: count, total }, Some(json!({ "card": c.name, "art": c.art, "count": before })), Some(json!({ "card": c.name, "art": c.art, "count": total })))
            }
            GiftRequest::Recipe { deck_id, include_cards, .. } => {
                let g = self.gifts().map_err(unavailable)?;
                let r = g.recipes.iter().find(|r| r.id == deck_id).ok_or_else(|| invalid("Choose a recipe from the catalogue."))?;
                let had = p.items.contains(&r.item);
                if had && !include_cards { return Err(MutationError::new(MutationErrorCode::AlreadyExists, "This player already has the recipe.")) }
                item = Some(r.item);
                if include_cards { for &row in &r.cards { *additions.entry(row).or_default() += 1; } }
                let body = if include_cards { format!("The host gave you the deck recipe {} and its {} cards. You will see them when you next come back to the map; the recipe is then on Load Deck's Official tab.", r.title, r.cards.len()) }
                    else { format!("The host gave you the deck recipe {} (not its cards). It is on Load Deck's Official tab from your next login, or after your next duel.", r.title) };
                letter = Some(("Recipe received", body));
                if include_cards { present = Some(Present::Deck { id: deck_id, cards: vec![] }); }
                (GiftOutcome::Recipe { deck_id, include_cards }, Some(json!({ "owned": had })), Some(json!({ "owned": true, "include_cards": include_cards })))
            }
            GiftRequest::Title { title_id, .. } => {
                let titles = self.titles().map_err(unavailable)?;
                if titles.name(title_id).is_none() { return Err(invalid("Choose a title from the catalogue.")) }
                item = Some(title_id);
                (GiftOutcome::Title { title_id, already_owned: p.items.contains(&title_id) }, Some(json!(p.items.contains(&title_id))), Some(json!(true)))
            }
        };
        for (&row, &count) in &additions {
            p.cards.get(&row).copied().unwrap_or_default().checked_add(count).filter(|n| *n <= 255)
                .ok_or_else(|| invalid("This gift exceeds the additional-card limit (255 per artwork)."))?;
        }
        let mut next = d.prepare_player(player_id, |p| {
            if let Some(item) = item { p.items.insert(item); }
            for (&row,&count) in &additions { *p.cards.entry(row).or_default() += count; }
            Ok(())
        }).map_err(invalid)?;
        let mail = if let Some((title, body)) = letter {
            let source = if let Some(h) = &hub { h.bp_mail() } else { match &self.data_dir { Some(dir) => Mailbox::try_load(&dir.join("mail.json")).map_err(MutationError::persistence)?, None => Mailbox::default() } };
            Some(source.prepare_host((player_id, &p.name), title, body).map_err(invalid)?)
        } else { None };
        let result = GiftResult { user_id, player_id, gift: outcome, request_id: id.into(), message: format!("Saved {operation} for player #{player_id}. Cards appear at the next map visit; recipes/titles at their next login or relevant screen refresh.") };
        let label = d.login_by_id(user_id).map_or_else(|| format!("#{user_id}"), |l| l.user.clone());
        let mut detail = match &input { Input::Named(op) => op.detail().unwrap_or_else(|| json!({})), _ => json!({}) };
        detail["who"] = json!(label); detail["player_id"] = json!(player_id); detail["request_id"] = json!(id);
        let target = match &input { Input::Named(op) => op.target(), _ => format!("#{player_id}") };
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by, origin: origin.into(), op: operation.clone(), target, before, after, detail: Some(detail) };
        let receipt = Receipt { operation, input: hash, result: json!(result) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; if let Some(mail) = &mail { mail.persist()?; } store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        if let (Some(h), Some(mail)) = (hub.as_mut(), mail) {
            let trunk = self.world.as_ref().filter(|_| !additions.is_empty()).and_then(|w| d.player(player_id).map(|p| crate::trade::trunk(w,p)));
            h.publish_gift(mail, player_id, trunk.as_ref(), present);
        }
        Ok(result)
    }

    /// Unlock all cards (Desktop's button for your own server): each artwork of every card in the catalogue topped up
    /// to `UNLOCK_COPIES` copies beyond the starting cards, for one player or every player, in one commit with a letter
    /// to each player who got cards, the audit row and the receipt; online players get their trunk at once.
    pub(super) fn unlock_cards(&self, player: Option<&str>, id: &str, actor: Actor<'_>, origin: &str) -> Result<Value, MutationError> {
        validate_id(id)?;
        let op = Op::UnlockCards { player: player.map(str::to_owned) };
        let mut hub = self.hub.as_ref().map(|h| h.locked());
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, Role::Admin)?;
        let operation = op.name(); let hash = fingerprint(&op)?;
        if let Some(result) = self.replay::<Value>(&identity.scope, id, &operation, &hash)? { return Ok(result) }
        let players: Vec<(u32, String)> = match player {
            Some(name) => { let player_id = d.player_id_named(name).ok_or_else(missing)?; vec![(player_id, d.player(player_id).ok_or_else(missing)?.name.clone())] }
            None => d.entries().flat_map(|(_, _, a)| a.players.iter().map(|p| (p.player_id, p.name.clone()))).collect(),
        };
        if players.is_empty() { return Err(invalid("There are no players yet: unlock the cards after the first sign-in.")) }
        let rows: Vec<u16> = self.gifts().map_err(unavailable)?.cards.iter().map(|c| c.row).collect();
        let mut next = (*d).clone();
        let mut mail = if let Some(h) = &hub { h.bp_mail() } else { match &self.data_dir { Some(dir) => Mailbox::try_load(&dir.join("mail.json")).map_err(MutationError::persistence)?, None => Mailbox::default() } };
        let (mut changed, mut added) = (vec![], 0u64);
        for (player_id, name) in &players {
            let mut more = 0u32;
            next = next.prepare_player(*player_id, |p| {
                for &row in &rows {
                    let have = p.cards.entry(row).or_default();
                    if *have < UNLOCK_COPIES { more += UNLOCK_COPIES - *have; *have = UNLOCK_COPIES; }
                }
                Ok(())
            }).map_err(invalid)?;
            if more > 0 {
                mail = mail.prepare_host((*player_id, name), "All cards unlocked", "The host unlocked every card for you: at least three of each, in every artwork. \
                    You will see them when you next come back to the map.".into()).map_err(invalid)?;
                changed.push(*player_id);
                added += u64::from(more);
            }
        }
        let message = match (changed.len(), players.len()) {
            (0, _) => "Every card was already unlocked.".to_owned(),
            (1, 1) => format!("Unlocked every card for {}: {added} cards added. They show at the next map visit.", players[0].1),
            (n, _) => format!("Unlocked every card for {n} players: {added} cards added. They show at the next map visit."),
        };
        let result = json!({ "players": changed.len(), "added": added, "message": message });
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by, origin: origin.into(), op: operation.clone(),
            target: op.target(), before: None, after: Some(json!({ "players": changed.len(), "added": added })), detail: Some(json!({ "request_id": id })) };
        let receipt = Receipt { operation, input: hash, result: result.clone() };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; mail.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        if let Some(h) = hub.as_mut() {
            for &player_id in &changed {
                let trunk = self.world.as_ref().and_then(|w| d.player(player_id).map(|p| crate::trade::trunk(w, p)));
                h.publish_gift(mail.clone(), player_id, trunk.as_ref(), None);
            }
        }
        Ok(result)
    }
}

/// Unlock all cards gives at least this many copies of each artwork: the most a deck may hold.
pub const UNLOCK_COPIES: u32 = 3;
