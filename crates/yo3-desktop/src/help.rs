//! Help (#31): the guide inside the launcher, a topic at a time, with Report a problem. The topics are `help.txt`:
//! `# id | Title`, then paragraphs and `- ` points. An error's notice opens Help at its own topic.
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Element, Length::Fill};
use yo3_skin::{BODY_BOLD, DISPLAY, Kind, color, pill};

const SOURCE: &str = include_str!("help.txt");

/// One paragraph or point of a topic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block { Paragraph(String), Point(String), Link(String, String) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topic { pub id: String, pub title: String, pub blocks: Vec<Block> }

/// The guide's topics, in order.
pub fn topics() -> Vec<Topic> {
    let mut topics: Vec<Topic> = Vec::new();
    for line in SOURCE.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        if let Some(head) = line.strip_prefix("# ") {
            let (id, title) = head.split_once(" | ").unwrap_or((head, head));
            topics.push(Topic { id: id.trim().into(), title: title.trim().into(), blocks: Vec::new() });
        } else if let Some(topic) = topics.last_mut() {
            topic.blocks.push(if let Some(point) = line.strip_prefix("- ") { Block::Point(point.into()) }
                else if let Some((label, url)) = line.strip_prefix("> ").and_then(|l| l.split_once(" | ")) { Block::Link(label.into(), url.into()) }
                else { Block::Paragraph(line.into()) });
        }
    }
    topics
}

#[derive(Clone, Debug)]
pub enum Message {
    Topic(String),
    /// Handled by the window.
    Report,
    /// Handled by the window: a web page.
    Open(String),
    Close,
}

pub struct State { topics: Vec<Topic>, pub current: String }

impl State {
    /// Help at topic `id` (the first topic when it is unknown).
    pub fn at(id: Option<&str>) -> Self {
        let topics = topics();
        let current = id.filter(|id| topics.iter().any(|t| t.id == *id)).map(str::to_owned).unwrap_or_else(|| topics[0].id.clone());
        Self { topics, current }
    }

    pub fn update(&mut self, message: &Message) { if let Message::Topic(id) = message { self.current = id.clone() } }

    pub fn view(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(9);
        for topic in &self.topics {
            list = list.push(if topic.id == self.current {
                Element::from(row![container(Space::new().width(4).height(18)).style(|_| container::Style { background: Some(color::NAVY.into()), ..container::Style::default() }),
                    text(&topic.title).size(15).font(BODY_BOLD).color(color::NAVY)].spacing(8))
            } else {
                row![Space::new().width(12), yo3_skin::link(topic.title.clone(), Message::Topic(topic.id.clone()), false)].into()
            });
        }
        let topic = self.topics.iter().find(|t| t.id == self.current).unwrap_or(&self.topics[0]);
        let mut body = column![text(&topic.title).font(DISPLAY).size(20).color(color::NAVY)].spacing(9);
        for block in &topic.blocks {
            body = body.push(match block {
                Block::Paragraph(p) => Element::from(text(p).size(15).color(color::INK)),
                Block::Point(p) => row![text("•").size(15).color(color::NAVY), text(p).size(15).color(color::INK).width(Fill)].spacing(8).into(),
                Block::Link(label, url) => yo3_skin::link(label.clone(), Message::Open(url.clone()), false),
            });
        }
        let buttons = row![
            pill("Report a problem", Some(Message::Report), Kind::Primary, 180, 38),
            Space::new().width(Fill),
            pill("Close", Some(Message::Close), Kind::Primary, 120, 38),
        ].spacing(10);
        let content = column![
            row![
                container(scrollable(list).height(Fill)).width(230),
                container(scrollable(body.padding([0, 12])).height(Fill)).width(Fill).padding([4, 10]).style(|_| container::Style {
                    background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }),
            ].spacing(12).height(Fill),
            buttons,
        ].spacing(12);
        container(yo3_skin::panel(Some("Help"), content)).width(860).height(520).into()
    }
}

/// The topic for an error's notice, from its sentence.
pub fn topic_for(sentence: &str) -> &'static str {
    let lower = sentence.to_lowercase();
    if lower.contains("port") { "port" }
    else if lower.contains("repair") || lower.contains("rules engine") { "repair" }
    else if lower.contains("wine") { "wine" }
    else if lower.contains("game") && (lower.contains("start") || lower.contains("crash") || lower.contains("stopped") || lower.contains("exited")) { "game-start" }
    else if lower.contains("sign") || lower.contains("password") { "signin" }
    else { "report" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_has_words_and_an_error_finds_its_topic() {
        let all = topics();
        assert!(all.len() >= 10);
        for topic in &all { assert!(!topic.blocks.is_empty(), "{}", topic.id) }
        for (sentence, id) in [("Another program is using your server's port.", "port"), ("Some of YO3 Revival's files need repair.", "repair"),
            ("The game crashed.", "game-start"), ("Something went wrong.", "report"), ("Wine couldn't run the game.", "wine")] {
            assert_eq!(topic_for(sentence), id);
            assert!(all.iter().any(|t| t.id == id), "{id}");
        }
        assert_eq!(State::at(Some("nonsense")).current, "start");
        let community = all.iter().find(|t| t.id == "community").expect("a topic for talking with other players");
        assert!(community.blocks.contains(&Block::Link("Join the Discord".into(), "https://discord.gg/NAm2T87n6q".into())));
    }
}
