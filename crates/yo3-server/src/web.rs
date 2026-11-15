//! The pages behind the game's links to Konami's old website (`yugioh-online.net`, gone since 2012): the connection DLL
//! sends them here (`tools/mod/web-links.h`) as `/web/` followed by the old path, through the launcher's local relay.
//! Leaderboard (`v3/events/rank_info?country=`), the team ranking (`v3/events/ranking/team/`), the BOOSTERPASS page
//! (`v3/bpass/index/`), the user events (`v3/userevents/index`) and the title screen's Official Website (`/`). Each says
//! what this server has; a leaderboard from real results waits for a public server (issue #16).

/// The page for `path` (the old site's path, after `/web`): its HTML. Every path has one: an old page the server
/// doesn't know says so.
pub fn page(path: &str) -> String {
    let path = path.split('?').next().unwrap_or_default().trim_start_matches('/');
    let (title, body) = match path {
        "" | "index.html" => (
            "Yu-Gi-Oh! ONLINE 3 revival",
            "<p>You are playing on a private revival server run by your host. Konami's official website closed with the \
             game in 2012.</p>",
        ),
        p if p.starts_with("v3/events/rank_info") => (
            "Leaderboard",
            "<p>There is no leaderboard on this server yet. Your wins and losses against other players are in \
             <b>Main Menu &rarr; My Profile</b>.</p>",
        ),
        p if p.starts_with("v3/events/ranking/team") => (
            "Team ranking",
            "<p>There is no team ranking on this server yet. Your team is in <b>Main Menu &rarr; Team</b>.</p>",
        ),
        p if p.starts_with("v3/bpass") => (
            "BOOSTERPASS Points",
            "<p>BOOSTERPASS Points (BP) can't be bought here. Your host hands them out, or gives you a BOOSTERPASS code to \
             enter in <b>Main Menu &rarr; BOOSTERPASS</b>.</p>",
        ),
        p if p.starts_with("v3/userevents") => (
            "User events",
            "<p>User events are in the game: the Satellite Plaza on the map.</p>",
        ),
        _ => ("Not here", "<p>This page of Konami's old website isn't on this server.</p>"),
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{title}</title><style>\
         body{{font-family:sans-serif;max-width:40em;margin:3em auto;padding:0 1em;line-height:1.5;color:#1b2340;background:#f4f6fb}}\
         h1{{font-size:1.5em}}a{{color:#2547a8}}</style></head><body><h1>{title}</h1>{body}\
         <p><a href=\"https://github.com/usersina/yo3-revival\">About the revival project</a></p></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_old_link_has_a_page() {
        assert!(page("/").contains("<title>Yu-Gi-Oh! ONLINE 3 revival</title>"));
        assert!(page("/v3/events/rank_info?country=GB").contains("<title>Leaderboard</title>"));
        assert!(page("/v3/events/ranking/team/?country=GB").contains("<title>Team ranking</title>"));
        assert!(page("/v3/bpass/index/").contains("<title>BOOSTERPASS Points</title>"));
        assert!(page("/v3/bpass/index.html").contains("BOOSTERPASS code"));
        assert!(page("/v3/userevents/index").contains("Satellite Plaza"));
        assert!(page("/v3/pages/faq/index").contains("<title>Not here</title>"));
        assert!(!page("/v3/events/rank_info?country=<script>").contains("<script>"), "the query is never echoed");
    }
}
