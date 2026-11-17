//! The order of a summon's commands, as the client's summon animations need it and Konami's server sent it: rules
//! learnt from client crashes, each checked against the 228 duel logs Konami's own server wrote (2010-2012; the test
//! `tests/summon_order.rs` replays them through these rules, which must find nothing there).
//!
//! The commands, of DuelSummon (`0x5165A0`) and DuelScene (`0x514290`): Set `0x2A`/`0x2D`/`0x30` marks a Tribute,
//! Fusion or Synchro material {player, zone, uid, 0}, Run `0x2C`/`0x2F`/`0x32` plays its effect {player, zone, uid,
//! variant}, CardMove `0x1A` moves a card {uid, from, to}, 1015 {count, uid ×3} and 1016 {uid ×4} fill the material
//! list, RunFusion `0x3A` {player, uid, kind << 16 | count} plays the Fusion (kind 0 and 1) or Synchro (2) animation.
//! A uid in a Set or Run may carry flags above its low 16 bits (Konami's logs: 1 or 2, 41 times): they are ignored.
//!
//! The rules:
//! - **A Tribute or Material Run is followed by that card's move before the next Run.** Konami's logs never play two
//!   materials' effects ahead of their moves; a Fusion's materials from the hand played that way crashed the client
//!   (a null card model read at `0x4FC4A9`, issue #40). A Synchro's Tuning Runs may be played together at the first
//!   move (our translator does, the Tuner first: the engine may move the non-Tuner first; live-checked).
//! - **A Synchro tunes a Tuner first.** Tuning Run's variant has 1 in its second byte for the first Tuner, which starts
//!   the level sums again (DuelSummon `+0x64`, `+0x68`, `+0x6C`), 2 for another Tuner and 0 for the others; a first
//!   Tuner after another material dropped that one's level, and the Synchro animation crashed on the sums (live).
//! - **An animation has its material list.** RunFusion of kind 0-2 reads the list 1015/1016 filled since the last one
//!   (DuelFusion `0x5071C0` makes a card image of each, DuelSynchro `0x507940` walks it from the Tuner count on): its
//!   count is RunFusion's, and a Synchro's has a Tuner in it. Without the list the Synchro animation crashed (live).
//!   Kind 3 (Konami's logs, 11 times, count 0) is not checked.
use crate::wire::cmd;


/// Where a summon's commands are: what the next one may be.
#[derive(Clone, Debug, Default)]
pub struct SummonOrder {
    /// A Tribute or Material Run whose card hasn't moved yet: (the Run, the uid).
    waiting: Option<(u32, u32)>,
    /// A Synchro's tuning since the first Tuner: Tuners tuned, other materials tuned.
    tuners: u32,
    others: u32,
    /// The material list since the last animation: its count and the uids given so far.
    list: Option<(u32, Vec<u32>)>,
}

impl SummonOrder {
    /// Follows one command sent to the client: the rule it breaks, if any.
    pub fn command(&mut self, id: u32, [p1, p2, p3, p4]: [u32; 4]) -> Option<String> {
        match id {
            cmd::TRIBUTE_RUN | cmd::MATERIAL_RUN | cmd::TUNING_RUN => {
                let uid = p3 & 0xFFFF;
                let mut problem = self.waiting.filter(|&(_, w)| w != uid).map(|(run, w)| {
                    format!("Run {id:#x} of uid {uid} while uid {w}'s Run {run:#x} still waits for its move (each Run goes right before its card's move: the client crashed at 0x4FC4A9, #40)")
                });
                if id == cmd::TUNING_RUN {
                    match p4 >> 8 & 0xFF {
                        1 => {
                            if self.others > 0 {
                                problem.get_or_insert(format!(
                                    "the first Tuner (uid {uid}) is tuned after {} other material(s): it drops their levels from the Synchro's sums",
                                    self.others
                                ));
                            }
                            (self.tuners, self.others) = (1, 0);
                        }
                        2 => self.tuners += 1,
                        _ => {
                            if self.tuners == 0 {
                                problem.get_or_insert(format!("uid {uid} is tuned before any Tuner (the first Tuner starts the sums again)"));
                            }
                            self.others += 1;
                        }
                    }
                } else {
                    self.waiting = Some((id, uid));
                }
                problem
            }
            cmd::MOVE => {
                if self.waiting.is_some_and(|(_, w)| w == p1 & 0xFFFF) {
                    self.waiting = None;
                }
                None
            }
            cmd::MATERIAL_LIST => {
                let count = p1;
                self.list = Some((count, [p2, p3, p4].into_iter().take(count as usize).collect()));
                None
            }
            cmd::MATERIAL_LIST_MORE => match &mut self.list {
                Some((count, uids)) => {
                    let room = (*count as usize).saturating_sub(uids.len());
                    uids.extend([p1, p2, p3, p4].into_iter().take(room));
                    None
                }
                None => Some("1016 adds to a material list that 1015 never started".into()),
            },
            cmd::RUN_FUSION => {
                let (kind, count) = (p3 >> 16, p3 & 0xFFFF);
                let mut problem = self.waiting.map(|(run, w)| format!("RunFusion while uid {w}'s Run {run:#x} still waits for its move"));
                if kind <= 2 {
                    let listed = self.list.take();
                    match listed {
                        None => {
                            problem.get_or_insert(format!(
                                "RunFusion (kind {kind}, {count} materials) without a material list 1015: the animations read it"
                            ));
                        }
                        Some((n, uids)) if n != count || uids.len() as u32 != count => {
                            problem.get_or_insert(format!(
                                "RunFusion (kind {kind}) names {count} materials, the material list {n} with {} uid(s) given",
                                uids.len()
                            ));
                        }
                        Some(_) => {}
                    }
                    if kind == 2 && self.tuners == 0 {
                        problem.get_or_insert("a Synchro animation with no Tuner tuned".into());
                    }
                }
                *self = SummonOrder::default();
                problem
            }
            cmd::RUN_SUMMON | cmd::RUN_SP_SUMMON => {
                self.waiting.take().map(|(run, w)| format!("the summon ends with uid {w}'s Run {run:#x} still waiting for its move"))
            }
            _ => None,
        }
    }
}
