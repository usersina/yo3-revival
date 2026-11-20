//! The leaderboard and the team ranking (#16): the pages behind Main Menu's Leaderboard (`web.rs`) from everyone's real
//! results, which are the wins, losses and draws against other players (NPC duels are practice and don't count, as in
//! My Profile). Players are ordered by wins, then win rate, then fewer losses; a team's results are its members' added.
//! Players with equal results share a rank.

/// One player's results and where to find them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub country: String,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub team: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamRow {
    pub name: String,
    pub members: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Ranking {
    pub players: Vec<Row>,
    pub teams: Vec<TeamRow>,
}

/// The most rows a page shows.
pub const SHOWN: usize = 100;

/// Wins per duel played, in thousandths (the order's second key).
fn rate(wins: u32, losses: u32, draws: u32) -> u64 {
    let played = (wins + losses + draws) as u64;
    (wins as u64 * 1000).checked_div(played).unwrap_or(0)
}

impl Ranking {
    /// The ranking of `rows` (players with at least one duel against another player) and of the teams, each with the
    /// results of every member (`[wins, losses, draws]`, zeros for a member who hasn't dueled).
    pub fn build(rows: Vec<Row>, teams: Vec<(String, Vec<[u32; 3]>)>) -> Ranking {
        let mut players: Vec<Row> = rows.into_iter().filter(|r| r.wins + r.losses + r.draws > 0).collect();
        players.sort_by(|a, b| b.wins.cmp(&a.wins).then(rate(b.wins, b.losses, b.draws).cmp(&rate(a.wins, a.losses, a.draws)))
            .then(a.losses.cmp(&b.losses)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        let mut teams: Vec<TeamRow> = teams.into_iter().map(|(name, members)| TeamRow {
            name, members: members.len() as u32, wins: members.iter().map(|m| m[0]).sum(), losses: members.iter().map(|m| m[1]).sum(),
            draws: members.iter().map(|m| m[2]).sum(),
        }).filter(|t| t.wins + t.losses + t.draws > 0).collect();
        teams.sort_by(|a, b| b.wins.cmp(&a.wins).then(rate(b.wins, b.losses, b.draws).cmp(&rate(a.wins, a.losses, a.draws)))
            .then(a.losses.cmp(&b.losses)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ranking { players, teams }
    }

    /// The players of one country (an ISO code, any case), ranked among themselves.
    pub fn in_country(&self, code: &str) -> Vec<&Row> {
        self.players.iter().filter(|r| r.country.eq_ignore_ascii_case(code)).collect()
    }
}

/// The ranks of rows already in order: equal results share the first's rank (1, 2, 2, 4).
pub fn ranks(results: &[[u32; 3]]) -> Vec<usize> {
    let mut out: Vec<usize> = vec![];
    for (i, r) in results.iter().enumerate() {
        out.push(if i > 0 && results[i - 1] == *r { out[i - 1] } else { i + 1 });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, country: &str, wins: u32, losses: u32, draws: u32) -> Row {
        Row { name: name.into(), country: country.into(), wins, losses, draws, team: None }
    }

    #[test]
    fn players_are_ordered_by_wins_then_rate_then_fewer_losses() {
        let ranking = Ranking::build(vec![row("Mai", "", 5, 5, 0), row("Yugi", "", 5, 1, 0), row("Joey", "", 9, 9, 0), row("Idle", "", 0, 0, 0), row("Tea", "", 5, 1, 0)], vec![]);
        let names: Vec<&str> = ranking.players.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Joey", "Tea", "Yugi", "Mai"], "no duel, no rank; equal results by name");
    }

    #[test]
    fn a_countrys_players_are_ranked_among_themselves() {
        let ranking = Ranking::build(vec![row("A", "GB", 1, 0, 0), row("B", "JP", 9, 0, 0), row("C", "gb", 3, 0, 0)], vec![]);
        let gb: Vec<&str> = ranking.in_country("GB").iter().map(|r| r.name.as_str()).collect();
        assert_eq!(gb, ["C", "A"]);
    }

    #[test]
    fn a_teams_results_are_its_members_added() {
        let ranking = Ranking::build(vec![], vec![("Duo".into(), vec![[3, 1, 0], [2, 0, 1]]), ("Quiet".into(), vec![[0, 0, 0]]), ("Solo".into(), vec![[9, 0, 0]])]);
        let rows: Vec<(&str, u32, u32, u32, u32)> = ranking.teams.iter().map(|t| (t.name.as_str(), t.members, t.wins, t.losses, t.draws)).collect();
        assert_eq!(rows, [("Solo", 1, 9, 0, 0), ("Duo", 2, 5, 1, 1)], "a team that hasn't dueled isn't ranked");
    }

    #[test]
    fn equal_results_share_a_rank() {
        assert_eq!(ranks(&[[5, 1, 0], [5, 1, 0], [4, 0, 0], [3, 0, 0]]), [1, 1, 3, 4]);
    }
}
