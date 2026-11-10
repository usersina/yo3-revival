//! Diagnostic checkpoint schema 1. The DLL independently reads these words from the actual client.
//! Hashes are FNV-1a over little-endian u32 words, not addresses; docs/protocol.md records every native offset.
use super::*;
pub const CHECKPOINT: u32 = 0x5F0;
pub const SCHEMA: u32 = 1;
static SEQUENCE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub fn hash(words: &[u32]) -> u32 {
    words.iter().flat_map(|w| w.to_le_bytes()).fold(0x811C9DC5, |h, b| (h ^ b as u32).wrapping_mul(0x1000193))
}

impl ClientModel {
    /// Result scenes stop normal command dispatch. Check the final state before that transition is queued.
    pub fn checkpoint_into(&mut self, part: &mut Vec<Message>) {
        if part.is_empty() { return; }
        let at = part.iter().position(|m| m.get("duelRequestName") == Some(&Value::U32(0x11))
            && m.get("id") == Some(&Value::U32(crate::translate::DUEL_END))).unwrap_or(part.len());
        part.insert(at, self.checkpoint());
    }

    pub fn native_words(&self) -> [Vec<u32>; 2] {
        let mut inventory = vec![SCHEMA];
        let mut stats = vec![SCHEMA];
        for p in 0..2 {
            inventory.push(self.lp[p].map_or(u32::MAX, |v| v as u32));
            for zone in 0..=10 {
                let uid = self.slots[p][zone];
                inventory.push(uid.unwrap_or(0));
                if let Some(uid) = uid {
                    let place = self.places.get(&uid);
                    inventory.push(place.map_or(0, |at| p as u32 | (zone as u32) << 1 | (at.face_up as u32) << 14 | (at.defense as u32) << 15));
                    inventory.push(u32::from(!self.off_field.contains(&uid)));
                }
                if zone < 5 && uid.and_then(|u| self.places.get(&u)).is_some_and(|at| at.face_up) {
                    stats.extend_from_slice(&self.records.get(&(p, zone as u32)).copied().unwrap_or_default()[4..9]);
                }
                if let Some(uid) = uid {
                    let mut counters: Vec<_> = self.counters.iter().filter(|((card, _), n)| *card == uid && **n != 0)
                        .map(|((_, kind), n)| (*kind, *n as u32)).collect();
                    counters.sort_unstable();stats.push(counters.len() as u32);
                    for (kind, count) in counters { stats.extend([kind, count]); }
                }
            }
            inventory.push(self.hands[p].len() as u32);
            inventory.extend(&self.hands[p]);
            inventory.extend(self.piles[p].map(|v| v as u32));
        }
        [inventory, stats]
    }

    /// Sent only with --duel-check, after the checked batch. The required mod reports PASS/FAIL or an incomplete
    /// checkpoint; successful native validation requires an actual snapshot acknowledgement in the game log.
    pub fn checkpoint(&mut self) -> Message {
        // Unique across both seats and all duels in this server session, even when their state hashes are identical.
        let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let [inventory, stats] = self.native_words();
        let expected = [hash(&inventory), hash(&stats)];
        tracing::info!("[YO3 EXPECT] {}", serde_json::json!({"schema":SCHEMA,"sequence":sequence,"expected":expected}));
        crate::duel::command(CHECKPOINT, [SCHEMA, sequence, expected[0], expected[1]])
    }
}
