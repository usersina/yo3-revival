//! The leaderboard and the team ranking (#16): Main Menu's Leaderboard opens the host's page (`web.rs`), filled by
//! `Hub::ranking` from everyone's duels against other players. NPC duels (ids up to 256) are practice and don't count.
mod common;

use common::{Server, one, req_with};
use yo3_proto::Value;

fn text(v: &str) -> Value { Value::text(v) }

#[test]
fn the_ranking_counts_duels_between_players_and_a_team_adds_its_members() {
    let server = Server::new();
    let (mut yugi, mut kaiba, mut joey) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    // Yugi forms Millennium and Kaiba joins it (the team contract's steps); Joey stays out.
    one(yugi.send(req_with(0x2AF9, &[("team_name", text("Millennium"))])));
    let team = one(yugi.send(common::req(0x2AFB))).get("team_id").and_then(Value::as_u32).expect("team_id");
    one(yugi.send(req_with(0x2AFE, &[("to_id", Value::U32(1002))])));
    kaiba.pushed();
    one(kaiba.send(req_with(0x2B00, &[("to_id", Value::U32(1001)), ("team_id", Value::U32(team)), ("answer", Value::U32(2))])));
    {
        let mut hub = server.hub.lock().unwrap();
        // Yugi 2-1 against Joey, Kaiba 1-0 against Joey, Joey a draw against Kaiba; and wins over NPC 124.
        for (me, them, result) in [(1001, 1003, 1), (1001, 1003, 1), (1001, 1003, 2), (1002, 1003, 1), (1003, 1002, 3), (1001, 124, 1), (1003, 124, 1)] {
            hub.record_result(me, them, result);
        }
    }
    let ranking = server.hub.lock().unwrap().ranking();
    let rows: Vec<(&str, u32, u32, u32, Option<&str>)> = ranking.players.iter().map(|r| (r.name.as_str(), r.wins, r.losses, r.draws, r.team.as_deref())).collect();
    assert_eq!(rows, [("Yugi", 2, 1, 0, Some("Millennium")), ("Kaiba", 1, 0, 0, Some("Millennium")), ("Joey", 0, 0, 1, None)],
        "the NPC duels count for nobody; Joey's lost duels are the others' wins, not hers");
    assert_eq!(ranking.teams.len(), 1);
    let team = &ranking.teams[0];
    assert_eq!((team.name.as_str(), team.members, team.wins, team.losses, team.draws), ("Millennium", 2, 3, 1, 0));
    drop((kaiba.pushed(), joey.pushed()));
}
