//! How each NPC plays: its duel bot style (crates/yo3-engine's bot::Style), by character. The client names an NPC
//! (`GameData::npc_name`, the same for its three levels); the characters are ours to play, so these are our choices,
//! not the original NPCs' (Konami's logs don't record how their CPU decided). `aggression` -1..1 (holds back ..
//! attacks whenever it can), `caution` 0..1 (weighs threats, needs more to spend a card), `sees`: the opponent's
//! face-down monsters and Set cards (not a player wearing the Millennium Eye, `against`). Each era's hero has the Heart
//! of the Cards (`skills`: the engine's skills.lua): before each draw, the card he needs most goes on top of his Deck.
//! Each character is one record of `npc_characters.json` (its numbers, skills, brain and plan); its lines are in
//! `npc_lines.txt` (banter.rs).
use serde_json::{Value as Json, json};

/// One character's duel bot: a record of `npc_characters.json`, which holds every NPC's (the game's own and ours, by the
/// name the client gives it). A change to a character's numbers is measured before it lands (`task check:npcs`, the
/// arena) and the record's `note` says how.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Character {
    pub name: String,
    /// -1 (holds back) to 1 (attacks whenever it can).
    pub aggression: f64,
    /// 0 to 1: weighs threats, needs more to spend a card.
    pub caution: f64,
    /// Sees the opponent's face-down monsters and Set cards (Pegasus' Millennium Eye).
    #[serde(default)]
    pub sees: bool,
    /// The skills the engine gives it (skills.lua): `heart_of_the_cards` for each era's hero and the seers (Ishizu's
    /// Millennium Necklace, Sartorius' fortune telling, Z-one's future: measured 2026-11-27, 24 duels each, the same
    /// seeds: wins 13 to 16, 10 to 19 and 4 to 21).
    #[serde(default)]
    pub skills: Vec<String>,
    /// How it decides, when not by the look-ahead (the engine's bot::brain: `rules`, `placeholder`).
    #[serde(default)]
    pub brain: Option<String>,
    /// Its decks' game plan (the engine's bot::plan): the bosses it plays toward and the cards that bring them out.
    #[serde(default)]
    pub plan: Option<Json>,
    /// Why it plays as it does, and how a change was measured.
    #[serde(default)]
    pub note: String,
}

/// Every character with a bot style (`npc_characters.json`).
pub fn characters() -> &'static [Character] {
    static ALL: std::sync::LazyLock<Vec<Character>> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("npc_characters.json")).expect("npc_characters.json reads")
    });
    &ALL
}

/// The character named `name`, if it has a style.
pub fn character(name: &str) -> Option<&'static Character> {
    characters().iter().find(|c| c.name == name)
}

/// The Millennium Eye a player can wear: Bakura's Accessories (male 84, female 342 in the item catalogue), a golden
/// Eye over the left eye, worn as Facial Accessory 2 (avatar1 slot 9: slot n holds a part of catalogue category n).
pub const MILLENNIUM_EYE: [u16; 2] = [84, 342];
const EYE_SLOT: usize = 9;

/// Whether a player's appearance (`avatar1`, big-endian u16 slots) wears the Millennium Eye.
pub fn wears_millennium_eye(avatar: &[u8]) -> bool {
    let slot = avatar.get(EYE_SLOT * 2..EYE_SLOT * 2 + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    slot.is_some_and(|id| MILLENNIUM_EYE.contains(&id))
}

/// The NPC's style against a player who looks like `avatar`: a Millennium Item holder can't be read, so a character
/// who sees (Pegasus) doesn't against one wearing the Millennium Eye.
pub fn against(style: &Json, avatar: &[u8]) -> Json {
    let mut style = style.clone();
    if style["sees"] == true && wears_millennium_eye(avatar) {
        style["sees"] = Json::Bool(false);
    }
    style
}

/// The style of the NPC named `name` (crates/yo3-engine's bot::Style and its skills); an NPC not listed plays the
/// default (null).
pub fn style(name: &str) -> Json {
    character(name).map_or(Json::Null, |c| {
        let mut style = json!({ "aggression": c.aggression, "caution": c.caution, "sees": c.sees, "skills": c.skills });
        if let Some(brain) = &c.brain {
            style["brain"] = json!(brain);
        }
        if let Some(plan) = &c.plan {
            style["plan"] = plan.clone();
        }
        style
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seers_draw_what_they_need() {
        for seer in ["Ishizu Ishtar", "Sartorius", "Z-one"] {
            assert_eq!(style(seer)["skills"], json!(["heart_of_the_cards"]), "{seer}");
        }
        assert_eq!(style("Dartz")["skills"], json!([]));
    }

    #[test]
    fn every_character_reads_once_with_numbers_in_range() {
        let mut names = std::collections::HashSet::new();
        for c in characters() {
            assert!(names.insert(&c.name), "{} twice", c.name);
            assert!((-1.0..=1.0).contains(&c.aggression) && (0.0..=1.0).contains(&c.caution), "{}", c.name);
            assert!(c.skills.iter().all(|s| ["heart_of_the_cards", "destiny_draw"].contains(&s.as_str())), "{}", c.name);
        }
        assert_eq!(characters().len(), 59);
    }

    #[test]
    fn pegasus_sees_and_kaiba_attacks() {
        assert_eq!(style("Maximillion Pegasus")["sees"], true);
        assert_eq!(style("Seto Kaiba")["sees"], false);
        assert!(style("Seto Kaiba")["aggression"].as_f64().unwrap() > 0.5);
        assert_eq!(style("Someone Else"), Json::Null);
        for hero in ["Yami Yugi", "Yugi Muto", "Jaden Yuki", "Yusei Fudo"] {
            assert_eq!(style(hero)["skills"], json!(["heart_of_the_cards"]), "{hero}");
        }
        assert_eq!(style("Seto Kaiba")["skills"], json!([]));
    }

    #[test]
    fn the_millennium_eye_blinds_pegasus() {
        // Bakura's Accessories (84 male, 342 female) is the Millennium Eye over the left eye: Facial Accessory 2,
        // avatar1 slot 9 (big-endian u16s; slot n holds a part of catalogue category n).
        let wearing = |slot: usize, id: u16| {
            let mut avatar = vec![0u8; 82];
            avatar[slot * 2..slot * 2 + 2].copy_from_slice(&id.to_be_bytes());
            avatar
        };
        assert!(wears_millennium_eye(&wearing(9, 84)) && wears_millennium_eye(&wearing(9, 342)));
        assert!(!wears_millennium_eye(&wearing(9, 94)), "a Pirate Eyepatch is just a patch");
        assert!(!wears_millennium_eye(&wearing(2, 84)) && !wears_millennium_eye(&[]) && !wears_millennium_eye(&[0, 84]));
        let pegasus = style("Maximillion Pegasus");
        assert_eq!(against(&pegasus, &wearing(9, 84))["sees"], false);
        assert_eq!(against(&pegasus, &wearing(9, 84))["aggression"], pegasus["aggression"], "only his sight goes");
        assert_eq!(against(&pegasus, &[0; 82])["sees"], true);
        assert_eq!(against(&style("Seto Kaiba"), &wearing(9, 84)), style("Seto Kaiba"));
    }
}
