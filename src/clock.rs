use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use tokio::sync::mpsc;

use crate::{
    app::AppAction,
    clock::{error::ClockError, timer::Timer},
};

pub mod error;
pub mod timer;

type Result<T> = std::result::Result<T, error::ClockError>;

#[derive(Debug)]
pub struct Clock {
    timer: Timer,
    pub tasks: Vec<Task>,
    pub current_task_id: Option<usize>,
    app_action_tx: mpsc::Sender<AppAction>,
}

#[derive(Debug)]
pub struct Task {
    pub content: String,
    pub seconds: f64,
}

impl Clock {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>) -> Self {
        Self {
            timer: Timer::default(),
            tasks: Vec::new(),
            current_task_id: None,
            app_action_tx,
        }
    }

    pub fn current_task_seconds(&self) -> Option<f64> {
        self.tasks
            .get(self.current_task_id?)
            .map(|task| task.seconds)
    }

    pub async fn start_next_task(&mut self) -> Result<()> {
        let app_tx = self.app_action_tx.clone();
        let on_tick = move |left_seconds| {
            let app_tx = app_tx.clone();
            tokio::spawn(async move {
                let _ = app_tx
                    .send(AppAction::UpdateClockProgress(left_seconds))
                    .await;
            });
        };

        if self.current_task_id.is_none() {
            self.current_task_id = Some(0);
        }
        let task_seconds = self.current_task_seconds().ok_or(ClockError::NoTask)?;
        self.timer.start(task_seconds, on_tick).await?;

        self.current_task_id = self.current_task_id.map(|id| (id + 1) % self.tasks.len());
        Ok(())
    }

    pub fn render_with_current_seconds_left(
        &self,
        frame: &mut Frame,
        area: Rect,
        seconds_left: f64,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        if let Some(total_seconds) = self.current_task_seconds() {
            let ration = if total_seconds > 0.0 {
                (seconds_left / total_seconds).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let label = format!("{}s remaining", seconds_left);
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::BOTTOM))
                .gauge_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .ratio(ration)
                .label(label);
            frame.render_widget(gauge, layout[0]);
        } else {
            let paragraph =
                Paragraph::new("Not running").block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(paragraph, layout[0]);
        }

        let items: Vec<_> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let style = if Some(i) == self.current_task_id {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let content = format!(" [{}] {} ({}s)", i + 1, task.content, task.seconds);
                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title("Task List")
                    .border_set(border::ROUNDED),
            )
            .highlight_symbol(">> ");

        frame.render_widget(list, layout[1]);
    }
}

impl Clock {}
