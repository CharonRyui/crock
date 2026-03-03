use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

#[derive(Debug)]
pub struct HelpPane;

static HELP_LINES: OnceLock<Vec<Line>> = OnceLock::new();

macro_rules! help_line {
    ($key:expr, $description:expr) => {
        Line::from(vec![$key.bold(), ": ".italic(), $description.italic()])
    };
}

impl HelpPane {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Help / Keybindings ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let lines = help_lines();
        let text = Text::from(lines.clone());
        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }
}

fn help_lines() -> &'static Vec<Line<'static>> {
    HELP_LINES.get_or_init(|| {
        vec![
            help_line!("q", "quit"),
            help_line!("?", "toggle this pane"),
            help_line!("a", "add task"),
            help_line!("c", "continue next task"),
            help_line!("r", "run from the first task"),
        ]
    })
}
