//! The pages behind the game's links to Konami's old website (`yugioh-online.net`, gone since 2012): the connection DLL
//! sends them here (`tools/mod/web-links.h`) as `/web/` followed by the old path, through the launcher's local relay.
//! Leaderboard (`v3/events/rank_info?country=`), the team ranking (`v3/events/ranking/team/`), the BOOSTERPASS page
//! (`v3/bpass/index/`), the user events (`v3/userevents/index`) and the title screen's Official Website (`/`). Each says
//! what this server has; the leaderboard and the team ranking are filled from everyone's real results (`ranking.rs`, issue #16).

use crate::ranking::{Ranking, Row, SHOWN, TeamRow, ranks};

/// Whether the page for `path` is filled from the results (the leaderboard and the team ranking).
pub fn needs_ranking(path: &str) -> bool {
    let path = path.split('?').next().unwrap_or_default().trim_start_matches('/');
    path.starts_with("v3/events/rank_info") || path.starts_with("v3/events/ranking/team")
}

/// `text` for HTML: a player's or team's name is the player's own.
fn escape(text: &str) -> String {
    text.chars().fold(String::new(), |mut out, c| {
        match c { '&' => out.push_str("&amp;"), '<' => out.push_str("&lt;"), '>' => out.push_str("&gt;"), '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"), c if c.is_control() => out.push(' '), c => out.push(c) }
        out
    })
}

/// The `country` of the query (`?country=GB`): two ASCII letters in capitals, else none. Nothing else of it is echoed.
fn country(path: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    let value = query.split('&').find_map(|p| p.strip_prefix("country="))?;
    (value.len() == 2 && value.bytes().all(|b| b.is_ascii_alphabetic())).then(|| value.to_ascii_uppercase())
}

fn rate(wins: u32, losses: u32, draws: u32) -> String {
    let played = wins + losses + draws;
    (wins * 100).checked_div(played).map_or_else(|| "-".into(), |p| format!("{p}%"))
}

/// The leaderboard: the players with a duel against another player, the best first.
fn leaderboard(ranking: &Ranking, country: Option<&str>) -> String {
    let all: Vec<&Row> = match country { Some(code) => ranking.in_country(code), None => ranking.players.iter().collect() };
    let scope = match country {
        Some(code) => format!("<p>Players of <b>{code}</b>. <a href=\"rank_info\">Everyone</a></p>"),
        None => "<p>Everyone on this server. The game's link opens your own country: add <code>?country=</code> and its code.</p>".into(),
    };
    if all.is_empty() {
        return format!("{scope}<p>Nobody has won or lost a duel against another player here yet. Your results are in <b>Main Menu &rarr; My Profile</b>.</p>");
    }
    let shown = &all[..all.len().min(SHOWN)];
    let ranked = ranks(&shown.iter().map(|r| [r.wins, r.losses, r.draws]).collect::<Vec<_>>());
    let rows: String = shown.iter().zip(ranked).map(|(r, rank)| format!(
        "<tr><td>{rank}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
        escape(&r.name), escape(r.team.as_deref().unwrap_or("")), r.country, r.wins, r.losses, r.draws, rate(r.wins, r.losses, r.draws))).collect();
    format!("{scope}<p>Wins, losses and draws against other players. Practice against the computer doesn't count.</p>\
        <table><tr><th>#</th><th>Player</th><th>Team</th><th>Country</th><th>Wins</th><th>Losses</th><th>Draws</th><th>Win rate</th></tr>{rows}</table>\
        {}", if all.len() > SHOWN { format!("<p>The best {SHOWN} of {} players.</p>", all.len()) } else { String::new() })
}

/// The team ranking: each team's members' results added.
fn team_ranking(ranking: &Ranking) -> String {
    if ranking.teams.is_empty() {
        return "<p>No team has won or lost a duel against another player yet. Your team is in <b>Main Menu &rarr; Team</b>.</p>".into();
    }
    let shown: &[TeamRow] = &ranking.teams[..ranking.teams.len().min(SHOWN)];
    let ranked = ranks(&shown.iter().map(|t| [t.wins, t.losses, t.draws]).collect::<Vec<_>>());
    let rows: String = shown.iter().zip(ranked).map(|(t, rank)| format!(
        "<tr><td>{rank}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
        escape(&t.name), t.members, t.wins, t.losses, t.draws, rate(t.wins, t.losses, t.draws))).collect();
    format!("<p>Each team's results are its members' added: duels against other players.</p>\
        <table><tr><th>#</th><th>Team</th><th>Members</th><th>Wins</th><th>Losses</th><th>Draws</th><th>Win rate</th></tr>{rows}</table>")
}

/// The page for `path` (the old site's path, after `/web`, with its query): its HTML. Every path has one: an old page the
/// server doesn't know says so. `ranking` is the results the leaderboard and the team ranking show (none: the page says it
/// can't read them).
pub fn page(path: &str, ranking: Option<&Ranking>) -> String {
    let wanted = country(path);
    let path = path.split('?').next().unwrap_or_default().trim_start_matches('/');
    let (title, body): (&str, String) = match path {
        "" | "index.html" => (
            "Yu-Gi-Oh! ONLINE 3 revival",
            "<p>You are playing on a private revival server run by your host. Konami's official website closed with the \
             game in 2012.</p>".into(),
        ),
        p if p.starts_with("v3/events/rank_info") => (
            "Leaderboard",
            ranking.map_or_else(|| "<p>The results can't be read right now.</p>".into(), |r| leaderboard(r, wanted.as_deref())),
        ),
        p if p.starts_with("v3/events/ranking/team") => (
            "Team ranking",
            ranking.map_or_else(|| "<p>The results can't be read right now.</p>".into(), team_ranking),
        ),
        p if p.starts_with("v3/bpass") => (
            "BOOSTERPASS Points",
            "<p>BOOSTERPASS Points (BP) can't be bought here. Your host hands them out, or gives you a BOOSTERPASS code to \
             enter in <b>Main Menu &rarr; BOOSTERPASS</b>.</p>".into(),
        ),
        p if p.starts_with("v3/userevents") => (
            "User events",
            "<p>User events are in the game: the Satellite Plaza on the map.</p>".into(),
        ),
        _ => ("Not here", "<p>This page of Konami's old website isn't on this server.</p>".into()),
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{title}</title><style>\
         body{{font-family:sans-serif;max-width:40em;margin:3em auto;padding:0 1em;line-height:1.5;color:#1b2340;background:#f4f6fb}}\
         h1{{font-size:1.5em}}a{{color:#2547a8}}table{{border-collapse:collapse;width:100%}}\
         th,td{{text-align:left;padding:.25em .6em;border-bottom:1px solid #d5dbec}}th{{background:#e6eaf6}}</style></head><body><h1>{title}</h1>{body}\
         <p><a href=\"https://github.com/usersina/yo3-revival\">About the revival project</a></p></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::Row;

    fn row(name: &str, country: &str, wins: u32, losses: u32) -> Row {
        Row { name: name.into(), country: country.into(), wins, losses, draws: 0, team: Some("Millennium".into()) }
    }

    #[test]
    fn each_old_link_has_a_page() {
        assert!(page("/", None).contains("<title>Yu-Gi-Oh! ONLINE 3 revival</title>"));
        assert!(page("/v3/events/rank_info?country=GB", None).contains("<title>Leaderboard</title>"));
        assert!(page("/v3/events/ranking/team/?country=GB", None).contains("<title>Team ranking</title>"));
        assert!(page("/v3/bpass/index/", None).contains("<title>BOOSTERPASS Points</title>"));
        assert!(page("/v3/bpass/index.html", None).contains("BOOSTERPASS code"));
        assert!(page("/v3/userevents/index", None).contains("Satellite Plaza"));
        assert!(page("/v3/pages/faq/index", None).contains("<title>Not here</title>"));
        assert!(!page("/v3/events/rank_info?country=<script>", None).contains("<script>"), "the query is never echoed");
        assert!(needs_ranking("/v3/events/rank_info?country=GB") && needs_ranking("v3/events/ranking/team/") && !needs_ranking("/"));
    }

    #[test]
    fn the_leaderboard_lists_the_players_and_their_names_are_escaped() {
        let ranking = Ranking::build(vec![row("Yugi", "GB", 5, 1), row("<b>Kaiba</b>", "JP", 9, 0), row("Joey", "GB", 5, 1)], vec![]);
        let all = page("/v3/events/rank_info", Some(&ranking));
        let (kaiba, yugi, joey) = (all.find("&lt;b&gt;Kaiba&lt;/b&gt;").expect("escaped"), all.find("Yugi").unwrap(), all.find("Joey").unwrap());
        assert!(kaiba < joey && joey < yugi || kaiba < yugi, "{all}");
        assert!(!all.contains("<b>Kaiba</b>"), "a name is never markup");
        assert!(all.contains("<td>1</td><td>&lt;b&gt;Kaiba"), "Kaiba is first");
        assert!(all.contains("<td>2</td><td>Joey") && all.contains("<td>2</td><td>Yugi"), "equal results share a rank");
        assert!(all.contains("83%"), "5 of 6 duels won");
        // The game's own link carries the player's country: only that country's players, ranked among themselves.
        let gb = page("/v3/events/rank_info?country=GB", Some(&ranking));
        assert!(gb.contains("Players of <b>GB</b>") && gb.contains("Yugi") && !gb.contains("Kaiba"), "{gb}");
        assert!(gb.contains("<td>1</td><td>Joey") || gb.contains("<td>1</td><td>Yugi"));
        let empty = page("/v3/events/rank_info?country=ZZ", Some(&ranking));
        assert!(empty.contains("Nobody has won or lost"), "{empty}");
    }

    #[test]
    fn the_team_ranking_lists_the_teams() {
        let ranking = Ranking::build(vec![], vec![("Millennium".into(), vec![[3, 1, 0], [2, 0, 0]]), ("Idle".into(), vec![[0, 0, 0]])]);
        let teams = page("/v3/events/ranking/team/?country=GB", Some(&ranking));
        assert!(teams.contains("<td>1</td><td>Millennium</td><td>2</td><td>5</td><td>1</td><td>0</td><td>83%</td>"), "{teams}");
        assert!(!teams.contains("Idle"));
        assert!(page("/v3/events/ranking/team/", Some(&Ranking::default())).contains("No team has won or lost"));
    }
}
