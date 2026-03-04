use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

#[derive(Debug)]
pub struct HelpPane;

static HELP_TEXT: OnceLock<Text<'static>> = OnceLock::new();

impl HelpPane {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Help / Keybindings ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let text = help_text();
        let paragraph = Paragraph::new(text.clone()).block(block);

        frame.render_widget(paragraph, area);
    }
}

fn help_text() -> &'static Text<'static> {
    HELP_TEXT.get_or_init(|| {
        let lines = vec![
            // Section: General
            Line::from(" General ".bold().bg(Color::Blue).fg(Color::White)),
            Line::from(vec![
                Span::raw("  "),
                "q".bold().cyan(),
                ": Quit / Back to Clock".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "?".bold().cyan(),
                ": Toggle Help".into(),
            ]),
            Line::from(""),
            // Section: Clock (Main View)
            Line::from(" Clock View ".bold().bg(Color::Blue).fg(Color::White)),
            Line::from(vec![
                Span::raw("  "),
                "p".bold().cyan(),
                ": Pause / Resume Timer".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "r".bold().cyan(),
                ": Start / Restart Current Task".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "t".bold().cyan(),
                ": Stop Current Task".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "e".bold().cyan(),
                ": Manage Tasks (Task List)".into(),
            ]),
            Line::from(""),
            // Section: Task Management
            Line::from(
                " Task List Management "
                    .bold()
                    .bg(Color::Blue)
                    .fg(Color::White),
            ),
            Line::from(vec![
                Span::raw("  "),
                "a".bold().cyan(),
                ": Add New Task".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "r".bold().cyan(),
                ": Edit Focused Task".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "d".bold().cyan(),
                ": Delete Focused Task".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "j / k".bold().cyan(),
                ": Move Focus Down / Up".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "Enter".bold().cyan(),
                ": Set Focused Task as Current".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "Esc / q".bold().cyan(),
                ": Back to Clock".into(),
            ]),
            Line::from(""),
            // Section: Task Input (Add/Edit)
            Line::from(
                " Task Input Dialog "
                    .bold()
                    .bg(Color::Blue)
                    .fg(Color::White),
            ),
            Line::from(vec![
                Span::raw("  "),
                "Tab".bold().cyan(),
                ": Switch Field (Name/Duration)".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "Enter".bold().cyan(),
                ": Confirm and Save".into(),
            ]),
            Line::from(vec![
                Span::raw("  "),
                "Esc".bold().cyan(),
                ": Cancel and Return".into(),
            ]),
        ];

        Text::from(lines)
    })
}
