use std::{num::ParseFloatError, sync::OnceLock};

use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};
use regex::Regex;
use thiserror::Error;
use tracing::instrument;
use tui_input::{Input, backend::crossterm::EventHandler};

static TIME_REGEX: OnceLock<Regex> = OnceLock::new();

use crate::clock::Task;

#[derive(Debug, Error)]
pub enum TaskInputError {
    #[error("empty content")]
    EmptyContent,
    #[error("empty time")]
    EmptyTime,
    #[error("invalid time")]
    InvalidTime(#[from] ParseFloatError),
}

type Result<T> = std::result::Result<T, TaskInputError>;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum TaskInputFocus {
    #[default]
    Content,
    Time,
}

#[derive(Debug, Default)]
pub struct TaskInput {
    content_input: Input,
    time_input: Input,
    focus: TaskInputFocus,
}

impl TaskInput {
    #[instrument(skip(self))]
    pub fn get_task(&mut self) -> Result<Task> {
        let content = self.content_input.value_and_reset();
        if content.is_empty() {
            return Err(TaskInputError::EmptyContent);
        }

        let time = self.time_input.value_and_reset();
        if time.is_empty() {
            return Err(TaskInputError::EmptyTime);
        }

        let mut seconds = 0.0;
        if let Some(caps) = time_regex().captures(&time) {
            if let Some(hour) = caps.get(1) {
                let hour: f64 = hour.as_str().parse().unwrap_or(0.0);
                seconds += hour * 3600.0;
            }
            if let Some(minute) = caps.get(2) {
                let minute: f64 = minute.as_str().parse().unwrap_or(0.0);
                seconds += minute * 60.0;
            }
            if let Some(second) = caps.get(3) {
                let second: f64 = second.as_str().parse().unwrap_or(0.0);
                seconds += second;
            }
        }

        let content = content.into();

        Ok(Task { content, seconds })
    }

    #[instrument(skip(self, frame, area))]
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ])
            .margin(1)
            .split(area);

        let content_style = if self.focus == TaskInputFocus::Content {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let content_block = Block::default()
            .borders(Borders::ALL)
            .title(" 1. Task Description ")
            .border_style(content_style);

        frame.render_widget(
            Paragraph::new(self.content_input.value()).block(content_block),
            layout[0],
        );

        let time_style = if self.focus == TaskInputFocus::Time {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let time_title = Line::from(vec!["2. Time".bold(), "(in __h__min__s format)".italic()]);
        let time_block = Block::default()
            .borders(Borders::ALL)
            .title(time_title)
            .border_style(time_style);

        frame.render_widget(
            Paragraph::new(self.time_input.value()).block(time_block),
            layout[1],
        );

        let help_text = Line::from(vec![
            "<Tab>".bold(),
            " Switch Focus | ".italic(),
            "<Enter>".bold(),
            " Submit | ".italic(),
            "<Esc>".bold(),
            " Cancel ".italic(),
        ])
        .fg(Color::Indexed(245));
        frame.render_widget(Paragraph::new(help_text), layout[2]);

        let active_chunk = if self.focus == TaskInputFocus::Content {
            layout[0]
        } else {
            layout[1]
        };
        let active_input = if self.focus == TaskInputFocus::Content {
            &self.content_input
        } else {
            &self.time_input
        };

        frame.set_cursor_position(Position::new(
            active_chunk.x + active_input.visual_cursor() as u16 + 1,
            active_chunk.y + 1,
        ));
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            TaskInputFocus::Content => TaskInputFocus::Time,
            TaskInputFocus::Time => TaskInputFocus::Content,
        }
    }

    pub fn handle_event(&mut self, evt: Event) {
        match self.focus {
            TaskInputFocus::Content => self.content_input.handle_event(&evt),
            TaskInputFocus::Time => self.time_input.handle_event(&evt),
        };
    }
}

fn time_regex() -> &'static Regex {
    TIME_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(?:(?P<hour>\d*\.?\d+)h)?(?:(?P<minute>\d*\.?\d+)min)?(?:(?P<second>\d*\.?\d+)s)?"#)
            .unwrap()
    })
}
