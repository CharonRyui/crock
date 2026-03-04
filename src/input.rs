use std::num::ParseFloatError;

use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use thiserror::Error;
use tracing::instrument;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{tasks::Task, utils::parse_time};

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
        let content = self.content_input.value().trim().to_string();
        if content.is_empty() {
            return Err(TaskInputError::EmptyContent);
        }

        let time = self.time_input.value().trim().to_string();
        if time.is_empty() {
            return Err(TaskInputError::EmptyTime);
        }

        let seconds = parse_time(&time);

        // Reset inputs only after successful parse
        self.content_input.reset();
        self.time_input.reset();
        self.focus = TaskInputFocus::Content;

        Ok(Task {
            content: content.into(),
            seconds,
        })
    }

    #[instrument(skip(self, frame, area))]
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::bordered()
            .title(Span::styled(
                " Task Details ",
                Style::default().cyan().bold(),
            ))
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Indexed(240)));

        frame.render_widget(block, area);

        let inner_area = area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 2,
        });

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Content input
                Constraint::Length(3), // Time input
                Constraint::Min(0),    // Spacer
                Constraint::Length(1), // Help footer
            ])
            .split(inner_area);

        // --- 1. Content Input ---
        let content_focused = self.focus == TaskInputFocus::Content;
        let content_style = if content_focused {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let content_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(Span::styled(" Description ", content_style))
            .border_style(content_style);

        frame.render_widget(
            Paragraph::new(self.content_input.value()).block(content_block),
            layout[0],
        );

        // --- 2. Time Input ---
        let time_focused = self.focus == TaskInputFocus::Time;
        let time_style = if time_focused {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let time_title = Line::from(vec![
            Span::styled(" Duration ", time_style),
            Span::styled(
                "(e.g. 25m, 1h 30m, 45s)",
                Style::default().fg(Color::Indexed(242)).italic(),
            ),
        ]);
        let time_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(time_title)
            .border_style(time_style);

        frame.render_widget(
            Paragraph::new(self.time_input.value()).block(time_block),
            layout[1],
        );

        // --- 3. Footer ---
        let footer = Line::from(vec![
            " Tab ".bold().cyan(),
            "Switch ".into(),
            " Enter ".bold().cyan(),
            "Confirm ".into(),
            " Esc ".bold().cyan(),
            "Cancel ".into(),
        ])
        .alignment(Alignment::Center);

        frame.render_widget(Paragraph::new(footer), layout[3]);

        // Cursor Management
        let (active_area, active_input) = if content_focused {
            (layout[0], &self.content_input)
        } else {
            (layout[1], &self.time_input)
        };

        frame.set_cursor_position(Position::new(
            active_area.x + active_input.visual_cursor() as u16,
            active_area.y + 1,
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
