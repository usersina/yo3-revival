//! The pages behind the game's links to Konami's old website (`yugioh-online.net`, gone since 2012): the connection DLL
//! sends them here (`tools/mod/web-links.h`) as `/web/` followed by the old path, through the launcher's local relay.
//! Leaderboard (`v3/events/rank_info?country=`), the team ranking (`v3/events/ranking/team/`), the BOOSTERPASS page
//! (`v3/bpass/index/`), the user events (`v3/userevents/index`) and the title screen's Official Website (`/`), plus the
//! revival's own pack list (`packs`, `packs/ID`, #73). Each says what this server has; the leaderboard and the team
//! ranking are filled from everyone's real results (`ranking.rs`, issue #16).
//!
//! The browser opens them at `http://127.0.0.1:PORT/session/COOKIE/web/...` (#74), and the launcher's helper shows them
//! sandboxed, without the server's headers: so every link here is relative (it keeps that prefix) and nothing redirects.

use crate::ranking::{Ranking, Row, SHOWN, TeamRow, ranks};
use crate::shop::PackList;

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

/// What a page shows of the server: its name, the results, the packs on sale and their price.
#[derive(Clone, Copy, Default)]
pub struct Site<'a> {
    /// The `server-name` setting.
    pub name: &'a str,
    /// The results the leaderboard and the team ranking show (none: the page says it can't read them).
    pub ranking: Option<&'a Ranking>,
    pub packs: &'a [PackList],
    /// BP a pack costs (the `pack-cost` setting).
    pub pack_cost: u32,
}

/// The pages in the navigation bar: their path (after `/web/`) and name.
const NAV: [(&str, &str); 6] = [("", "Home"), ("v3/events/rank_info", "Leaderboard"), ("v3/events/ranking/team/", "Teams"),
    ("packs", "Packs"), ("v3/bpass/index/", "BP"), ("v3/userevents/index", "Events")];

/// The link from the page at `path` to `/web/` itself: one `../` per directory the path is in, so every link stays
/// under the browser's `/session/COOKIE/web/` (an absolute link would lose it).
fn base(path: &str) -> String { "../".repeat(path.matches('/').count()) }

/// The pack list (#73): each pack on sale, linked to its cards.
fn pack_index(site: &Site) -> String {
    if site.packs.is_empty() { return "<p>This server has no booster packs on sale.</p>".into() }
    let rows: String = site.packs.iter().map(|p| format!("<li><a href=\"packs/{}\">{}</a> ({} cards)</li>", p.id, escape(&p.name),
        p.tiers.iter().map(Vec::len).sum::<usize>())).collect();
    format!("<p>The booster packs in the shop (the map's top-left tile &rarr; Purchase Pack), {} BP each. Open one to see \
        every card it can give. In the game's chat, <code>!pack NAME</code> lists a pack's cards and <code>!where CARD</code> \
        names the packs a card is in.</p><ul>{rows}</ul>", site.pack_cost)
}

/// One pack's cards by rarity, with the odds `shop::open` deals them at.
fn pack_page(site: &Site, pack: &PackList) -> String {
    let chances = crate::shop::fifth_card_chances(pack.tiers.len());
    let tiers: String = pack.tiers.iter().enumerate().map(|(i, cards)| {
        let odds = match i {
            0 if pack.tiers.len() == 1 => "all five cards of a pack".to_string(),
            0 => "four cards of every pack".to_string(),
            _ => format!("the fifth card {}% of the time", chances.get(i - 1).copied().unwrap_or(0)),
        };
        let names: String = cards.iter().map(|n| format!("<li>{}</li>", escape(n))).collect();
        format!("<h2>{} <small>({} cards: {odds})</small></h2><ul class=\"cards\">{names}</ul>", crate::shop::rarity(i, pack.tiers.len()), cards.len())
    }).collect();
    let about = if pack.about.is_empty() { String::new() } else { format!("<p>{}</p>", escape(&pack.about)) };
    format!("<p><a href=\"../packs\">All packs</a></p>{about}<p>A pack of five cards costs {} BP. Rarely a card comes in another \
        of its artworks. The rarity names are this server's reading of the game's pack files.</p>{tiers}", site.pack_cost)
}

/// The page for `path` (the old site's path, after `/web`, with its query): its HTML. Every path has one: an old page the
/// server doesn't know says so.
pub fn page(path: &str, site: &Site) -> String {
    let wanted = country(path);
    let path = path.split('?').next().unwrap_or_default().trim_start_matches('/');
    let pack = path.strip_prefix("packs/").and_then(|id| id.trim_end_matches('/').parse::<u32>().ok())
        .and_then(|id| site.packs.iter().find(|p| p.id == id));
    let (title, body): (String, String) = if let Some(pack) = pack { (pack.name.clone(), pack_page(site, pack)) } else { match path {
        "" | "index.html" => (
            "Welcome".into(),
            format!("<p>You are playing on <b>{}</b>, a Yu-Gi-Oh! ONLINE 3 revival server run by its host. Konami's official \
             website closed with the game in 2012; these pages show what this server has.</p><ul>\
             <li><a href=\"v3/events/rank_info\">Leaderboard</a>: the players with the best results against other players.</li>\
             <li><a href=\"v3/events/ranking/team/\">Teams</a>: each team's results.</li>\
             <li><a href=\"packs\">Packs</a>: every card each booster pack can give, before you buy.</li>\
             <li><a href=\"v3/bpass/index/\">BP</a>: how BOOSTERPASS Points come in here.</li>\
             <li><a href=\"v3/userevents/index\">Events</a>: where tournaments and official events are.</li></ul>\
             <p>These pages work while your game is open: the launcher shows them for the server you joined.</p>", escape(site.name)),
        ),
        p if p.starts_with("v3/events/rank_info") => (
            "Leaderboard".into(),
            site.ranking.map_or_else(|| "<p>The results can't be read right now.</p>".into(), |r| leaderboard(r, wanted.as_deref())),
        ),
        p if p.starts_with("v3/events/ranking/team") => (
            "Team ranking".into(),
            site.ranking.map_or_else(|| "<p>The results can't be read right now.</p>".into(), team_ranking),
        ),
        "packs" | "packs/" => ("Booster packs".into(), pack_index(site)),
        p if p.starts_with("packs/") => ("Not here".into(), "<p>That pack isn't on sale on this server. <a href=\"../packs\">All packs</a></p>".into()),
        p if p.starts_with("v3/bpass") => (
            "BOOSTERPASS Points".into(),
            format!("<p>BOOSTERPASS Points (BP) buy packs, structure decks, protectors and avatar items in the shop. They can't be \
             bought with money: this server is free.</p><ul>\
             <li>Every account starts with {} BP.</li>\
             <li>Every duel pays: {} BP for a win, {} for a draw and {} for a loss, against players and the computer alike (not \
             between accounts played from the same PC or network).</li>\
             <li>Official events and the Championship can pay prizes, sent as a D-Mail.</li>\
             <li>Your host may hand out BP, or a BOOSTERPASS code to enter in <b>Main Menu &rarr; BOOSTERPASS</b>.</li></ul>\
             <p>A pack costs {} BP: see what each one gives under <a href=\"{}packs\">Packs</a>.</p>",
             crate::accounts::START_BP, crate::shop::duel_bp(1), crate::shop::duel_bp(3), crate::shop::duel_bp(2), site.pack_cost, base(p)),
        ),
        p if p.starts_with("v3/userevents") => (
            "Events".into(),
            "<ul><li><b>User events</b> are in the game: the Satellite Plaza on the map. Create User Event makes a knockout \
             tournament your friends join with Participate.</li>\
             <li><b>Official events</b> are in the Stadium (the map's Duelking Titlematch tile): Gunslinger, Duelist Trial, \
             Evolution and the other events your host runs.</li>\
             <li><b>The Championship</b> is the map's Tournament tile, on the day your host schedules it.</li></ul>".into(),
        ),
        _ => ("Not here".into(), "<p>This page of Konami's old website isn't on this server.</p>".into()),
    } };
    let to_web = base(path);
    let nav: String = NAV.iter().map(|(link, name)| format!("<a href=\"{to_web}{link}\">{name}</a>")).collect::<Vec<_>>().join(" ");
    let server = escape(site.name);
    let title = escape(&title);
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\">\
         <title>{title} - {server}</title><style>\
         body{{font-family:sans-serif;max-width:46em;margin:0 auto;padding:0 1em 2em;line-height:1.5;color:#1b2340;background:#f4f6fb}}\
         header{{border-bottom:2px solid #2547a8;padding:1em 0 .5em;margin-bottom:1em}}header b{{font-size:1.1em}}\
         nav a{{margin-right:.9em;white-space:nowrap}}h1{{font-size:1.5em}}h2{{font-size:1.15em;margin-top:1.4em}}h2 small{{font-weight:normal;color:#56607a}}\
         a{{color:#2547a8}}table{{border-collapse:collapse;width:100%}}\
         th,td{{text-align:left;padding:.25em .6em;border-bottom:1px solid #d5dbec}}th{{background:#e6eaf6}}\
         ul.cards{{columns:2 16em}}footer{{margin-top:2em;color:#56607a;font-size:.9em}}</style></head><body>\
         <header><b>{server}</b><nav>{nav}</nav></header><h1>{title}</h1>{body}\
         <footer>A Yu-Gi-Oh! ONLINE 3 revival server. <a href=\"https://github.com/usersina/yo3-revival\">About the revival project</a></footer></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::Row;

    fn packs() -> Vec<PackList> {
        vec![PackList { id: 7, name: "Legend of <Blue> Eyes".into(), about: "Season 1 & more.".into(), tiers: vec![vec!["Kuriboh".into(), "Mystical Elf".into()], vec!["Dark Magician".into()], vec!["Blue-Eyes White Dragon".into()]] }]
    }

    fn site(ranking: Option<&Ranking>) -> Site<'_> { Site { name: "Duel Arena", ranking, packs: &[], pack_cost: 100 } }

    /// `href` as the browser resolves it on the page at `at` (an absolute path): only relative links and `..`, as the
    /// pages use; anything else (`/x`, `//x`, `http:`…) is returned as is, outside every prefix.
    fn resolve(at: &str, href: &str) -> String {
        if href.starts_with('/') || href.contains(':') { return href.to_string() }
        let (href, query) = href.split_once('?').map_or((href, ""), |(h, q)| (h, q));
        let mut parts: Vec<&str> = at.split('/').collect();
        parts.pop();
        for piece in href.split('/') {
            match piece { ".." => { parts.pop(); } "." => {} p => parts.push(p) }
        }
        let path = parts.join("/");
        if query.is_empty() { path } else { format!("{path}?{query}") }
    }

    /// Every link of a page (its `href`s).
    fn links(html: &str) -> Vec<&str> {
        html.split("href=\"").skip(1).filter_map(|rest| rest.split('"').next()).collect()
    }

    fn row(name: &str, country: &str, wins: u32, losses: u32) -> Row {
        Row { name: name.into(), country: country.into(), wins, losses, draws: 0, team: Some("Millennium".into()) }
    }

    #[test]
    fn each_old_link_has_a_page() {
        assert!(page("/", &site(None)).contains("<title>Welcome - Duel Arena</title>"));
        assert!(page("/v3/events/rank_info?country=GB", &site(None)).contains("<title>Leaderboard - "));
        assert!(page("/v3/events/ranking/team/?country=GB", &site(None)).contains("<title>Team ranking - "));
        assert!(page("/v3/bpass/index/", &site(None)).contains("<title>BOOSTERPASS Points - "));
        assert!(page("/v3/bpass/index.html", &site(None)).contains("BOOSTERPASS code"));
        assert!(page("/v3/userevents/index", &site(None)).contains("Satellite Plaza"));
        assert!(page("/v3/pages/faq/index", &site(None)).contains("<title>Not here - "));
        assert!(!page("/v3/events/rank_info?country=<script>", &site(None)).contains("<script>"), "the query is never echoed");
        assert!(needs_ranking("/v3/events/rank_info?country=GB") && needs_ranking("v3/events/ranking/team/") && !needs_ranking("/"));
    }

    #[test]
    fn the_leaderboard_lists_the_players_and_their_names_are_escaped() {
        let ranking = Ranking::build(vec![row("Yugi", "GB", 5, 1), row("<b>Kaiba</b>", "JP", 9, 0), row("Joey", "GB", 5, 1)], vec![]);
        let all = page("/v3/events/rank_info", &site(Some(&ranking)));
        let (kaiba, yugi, joey) = (all.find("&lt;b&gt;Kaiba&lt;/b&gt;").expect("escaped"), all.find("Yugi").unwrap(), all.find("Joey").unwrap());
        assert!(kaiba < joey && joey < yugi || kaiba < yugi, "{all}");
        assert!(!all.contains("<b>Kaiba</b>"), "a name is never markup");
        assert!(all.contains("<td>1</td><td>&lt;b&gt;Kaiba"), "Kaiba is first");
        assert!(all.contains("<td>2</td><td>Joey") && all.contains("<td>2</td><td>Yugi"), "equal results share a rank");
        assert!(all.contains("83%"), "5 of 6 duels won");
        // The game's own link carries the player's country: only that country's players, ranked among themselves.
        let gb = page("/v3/events/rank_info?country=GB", &site(Some(&ranking)));
        assert!(gb.contains("Players of <b>GB</b>") && gb.contains("Yugi") && !gb.contains("Kaiba"), "{gb}");
        assert!(gb.contains("<td>1</td><td>Joey") || gb.contains("<td>1</td><td>Yugi"));
        let empty = page("/v3/events/rank_info?country=ZZ", &site(Some(&ranking)));
        assert!(empty.contains("Nobody has won or lost"), "{empty}");
    }

    #[test]
    fn the_team_ranking_lists_the_teams() {
        let ranking = Ranking::build(vec![], vec![("Millennium".into(), vec![[3, 1, 0], [2, 0, 0]]), ("Idle".into(), vec![[0, 0, 0]])]);
        let teams = page("/v3/events/ranking/team/?country=GB", &site(Some(&ranking)));
        assert!(teams.contains("<td>1</td><td>Millennium</td><td>2</td><td>5</td><td>1</td><td>0</td><td>83%</td>"), "{teams}");
        assert!(!teams.contains("Idle"));
        assert!(page("/v3/events/ranking/team/", &site(Some(&Ranking::default()))).contains("No team has won or lost"));
    }

    /// #74: the browser opens a page under the launch cookie's `/session/COOKIE/web/`; every link of every page, from any
    /// depth, stays under it (the project's own page on GitHub is the one link out).
    #[test]
    fn every_link_stays_under_the_sessions_prefix() {
        let packs = packs();
        let site = Site { packs: &packs, ..site(None) };
        let prefix = "/session/abc/web/";
        for path in ["", "index.html", "v3/events/rank_info?country=GB", "v3/events/ranking/team/", "v3/bpass/index/", "v3/bpass/index.html",
            "v3/userevents/index", "packs", "packs/", "packs/7", "packs/7/", "packs/99", "v3/pages/faq/index", "a/b/c/d/e"] {
            let html = page(path, &site);
            let at = format!("{prefix}{}", path.split('?').next().unwrap());
            for href in links(&html).into_iter().filter(|h| *h != "https://github.com/usersina/yo3-revival") {
                let to = resolve(&at, href);
                assert!(to.starts_with(prefix), "{path}: {href} -> {to}");
            }
            for (link, name) in NAV {
                assert!(links(&html).iter().any(|h| resolve(&at, h) == format!("{prefix}{link}")), "{path}: the bar's {name}");
            }
        }
    }

    #[test]
    fn the_server_name_heads_every_page_escaped() {
        let site = Site { name: "<i>Duel</i> & Co", ..site(None) };
        let html = page("v3/bpass/index/", &site);
        assert!(html.contains("<b>&lt;i&gt;Duel&lt;/i&gt; &amp; Co</b>") && !html.contains("<i>Duel"), "{html}");
        assert!(html.contains("can't be bought with money") && html.contains("5000 BP") && html.contains("100 BP for a win"), "{html}");
    }

    /// #73: before buying, the Packs page lists each pack and each pack's page every card, rarest named, escaped.
    #[test]
    fn the_packs_pages_list_every_card_by_rarity() {
        let packs = packs();
        let site = Site { packs: &packs, pack_cost: 250, ..site(None) };
        let index = page("packs", &site);
        assert!(index.contains("<a href=\"packs/7\">Legend of &lt;Blue&gt; Eyes</a> (4 cards)") && index.contains("250 BP"), "{index}");
        let one = page("packs/7", &site);
        assert!(one.contains("<title>Legend of &lt;Blue&gt; Eyes - Duel Arena</title>"), "{one}");
        for card in ["Kuriboh", "Mystical Elf", "Dark Magician", "Blue-Eyes White Dragon"] { assert!(one.contains(&format!("<li>{card}</li>")), "{card}") }
        assert!(one.contains("Common <small>(2 cards: four cards of every pack)") && one.contains("Rare <small>(1 cards: the fifth card 67% of the time)")
            && one.contains("Super Rare <small>(1 cards: the fifth card 33% of the time)"), "{one}");
        assert!(one.contains("<p>Season 1 &amp; more.</p>"), "the shop's description");
        assert!(page("packs/8", &site).contains("isn't on sale"));
        assert!(page("packs", &Site { packs: &[], ..site }).contains("no booster packs"));
    }
}
