//! How each NPC plays: its duel bot style (crates/yo3-engine's bot::Style), by character. The client names an NPC
//! (`GameData::npc_name`, the same for its three levels); the characters are ours to play, so these are our choices,
//! not the original NPCs' (Konami's logs don't record how their CPU decided). `aggression` -1..1 (holds back ..
//! attacks whenever it can), `caution` 0..1 (weighs threats, needs more to spend a card), `sees`: the opponent's
//! face-down monsters and Set cards (not a player wearing the Millennium Eye, `against`). Each era's hero has the Heart of the Cards (`skills`: the engine's
//! skills.lua): before each draw, the card he needs most goes on top of his Deck.
use serde_json::{Value as Json, json};

/// The heroes: they draw the card they need (`heart_of_the_cards`).
pub(crate) const HEROES: &[&str] = &["Yami Yugi", "Yugi Muto", "Jaden Yuki", "Yusei Fudo"];

/// (name, aggression, caution, sees)
pub(crate) const CHARACTERS: &[(&str, f64, f64, bool)] = &[
    ("Yami Yugi", 0.2, 0.3, false),
    ("Seto Kaiba", 0.9, 0.0, false),
    ("Joey Wheeler", 0.6, 0.0, false),
    ("Mai Valentine", 0.4, 0.2, false),
    ("Bandit Keith", 0.5, 0.0, false),
    // The Millennium Eye: he reads your mind.
    ("Maximillion Pegasus", 0.2, 0.3, true),
    ("Yami Bakura", 0.3, 0.4, false),
    ("Yami Marik", 0.7, 0.0, false),
    ("Yugi Muto", -0.2, 0.5, false),
    ("Jaden Yuki", 0.5, 0.1, false),
    ("Chazz Princeton", 0.6, 0.0, false),
    ("Alexis Rhodes", 0.2, 0.3, false),
    ("Syrus Truesdale", -0.3, 0.6, false),
    ("Hell Kaiser", 1.0, 0.0, false),
    ("Vellian Crowler", 0.3, 0.2, false),
    ("Aster Phoenix", 0.4, 0.2, false),
    ("Jesse Andersen", 0.2, 0.3, false),
    ("Yusei Fudo", 0.3, 0.3, false),
    ("Jack Atlas", 0.9, 0.0, false),
    ("Akiza Izinski", 0.5, 0.1, false),
    ("Leo", 0.6, 0.0, false),
    ("Luna", -0.2, 0.6, false),
    ("Crow Hogan", 0.6, 0.1, false),
    ("Carly Carmine", 0.0, 0.2, false),
    ("Kalin Kessler", 0.5, 0.2, false),
    // Custom NPCs (engine/added/npcs.json). Dartz: patient behind his walls until the Seal is down.
    ("Dartz", 0.3, 0.4, false),
];

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

/// The style of the NPC named `name`; an NPC not listed plays the default (null).
pub fn style(name: &str) -> Json {
    let skills: &[&str] = if HEROES.contains(&name) { &["heart_of_the_cards"] } else { &[] };
    CHARACTERS.iter().find(|(n, ..)| *n == name).map_or(Json::Null, |(_, aggression, caution, sees)| {
        json!({ "aggression": aggression, "caution": caution, "sees": sees, "skills": skills })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
