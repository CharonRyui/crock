use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph},
};
use tokio::sync::mpsc;
use tracing::instrument;
use tui_big_text::{BigText, PixelSize};

use crate::{
    app::{AppAction, ClockAppAction},
    clock::{error::ClockError, timer::Timer},
    tasks::Task,
    utils::format_time,
};

pub mod error;
pub mod timer;

type Result<T> = std::result::Result<T, ClockError>;

#[derive(Debug, Default)]
pub struct ClockState {
    pub seconds_left: Option<f64>,
    pub is_paused: bool,
    pub current_task: Option<Task>,
    pub next_task: Option<Task>,
}

#[derive(Debug)]
pub struct Clock {
    timer: Arc<Timer>,
    app_action_tx: mpsc::Sender<AppAction>,
}

impl Clock {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>) -> (Self, ClockState) {
        (
            Self {
                timer: Arc::default(),
                app_action_tx,
            },
            ClockState {
                is_paused: true,
                ..Default::default()
            },
        )
    }

    #[instrument(skip(self), err)]
    pub async fn run_task(&self, task: Task) -> Result<()> {
        let app_tx = self.app_action_tx.clone();
        let on_tick = move |seconds_left| {
            let app_tx = app_tx.clone();
            tokio::spawn(async move {
                let _ = app_tx
                    .send(AppAction::Clock(ClockAppAction::UpdateSecondsLeft(
                        seconds_left,
                    )))
                    .await;
            });
        };

        self.app_action_tx
            .send(AppAction::Clock(ClockAppAction::UpdateSecondsLeft(
                task.seconds,
            )))
            .await?;
        self.timer.run(task.seconds, on_tick).await?;

        self.app_action_tx
            .send(AppAction::Clock(ClockAppAction::TimerFinished(Some(task))))
            .await?;

        Ok(())
    }

    pub async fn toggle_pause(&self) {
        if self.timer.is_running().await {
            self.timer.pause_run().await;
        } else {
            self.timer.continue_run().await;
        }
    }

    pub async fn kill_current_task(&self) -> Result<()> {
        self.timer.stop_run().await;
        self.app_action_tx
            .send(AppAction::Clock(ClockAppAction::TimerFinished(None)))
            .await?;
        Ok(())
    }

    #[instrument(skip(self, frame, area))]
    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &ClockState) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status bar
                Constraint::Min(0),    // Main timer
                Constraint::Length(3), // Next task
                Constraint::Length(1), // Help hint
            ])
            .split(area);

        // --- 1. Status Bar ---
        let status_color = if state.is_paused {
            Color::Yellow
        } else {
            Color::Green
        };
        let status_text = if state.is_paused {
            " PAUSED "
        } else {
            " RUNNING "
        };
        let status_bar = Line::from(vec![
            Span::styled(
                " CROCK ",
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            ),
            Span::raw(" "),
            Span::styled(
                status_text,
                Style::default().bg(status_color).fg(Color::Black).bold(),
            ),
        ]);
        frame.render_widget(Paragraph::new(status_bar), main_layout[0]);

        // --- 2. Main Content ---
        if let Some(task) = &state.current_task {
            let timer_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40), // Task Name
                    Constraint::Percentage(40), // Big Timer
                    Constraint::Percentage(20), // Progress Bar
                ])
                .split(main_layout[1]);

            // Task Description
            let task_name = Paragraph::new(task.content.to_string())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan).bold());
            frame.render_widget(task_name, timer_area[0]);

            // Big Timer
            if let Some(seconds_left) = state.seconds_left {
                let time_str = format_time(seconds_left);
                let big_timer = BigText::builder()
                    .pixel_size(PixelSize::Full)
                    .style(Style::new().fg(status_color))
                    .lines(vec![time_str.into()])
                    .centered()
                    .build();
                frame.render_widget(big_timer, timer_area[1]);

                // Progress Gauge
                let ratio = (seconds_left / task.seconds).clamp(0.0, 1.0);
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(status_color).bg(Color::DarkGray))
                    .ratio(ratio)
                    .label(format!("{:.1}%", ratio * 100.0))
                    .use_unicode(true);
                frame.render_widget(gauge, timer_area[2]);
            }
        } else {
            let welcome = Paragraph::new(
                "\n\nNo Active Task\n\nPress 'e' to manage tasks\nPress 'r' to start first task",
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray).italic());
            frame.render_widget(welcome, main_layout[1]);
        }

        // --- 3. Next Up ---
        let next_block = Block::bordered()
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Indexed(240)));

        let next_content = if let Some(next) = &state.next_task {
            Line::from(vec![
                Span::styled(" NEXT: ", Style::default().fg(Color::DarkGray)),
                Span::styled(next.content.to_string(), Style::default().bold()),
                Span::styled(
                    format!(" ({})", format_time(next.seconds)),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            Line::from(Span::styled(
                " No tasks in queue ",
                Style::default().fg(Color::DarkGray),
            ))
        };
        frame.render_widget(
            Paragraph::new(next_content).block(next_block),
            main_layout[2],
        );

        // --- 4. Help Hint ---
        let help_hint = Line::from(vec![
            " p ".bold().cyan(),
            "Pause ".into(),
            " r ".bold().cyan(),
            "Run ".into(),
            " t ".bold().cyan(),
            "Stop ".into(),
            " e ".bold().cyan(),
            "Tasks ".into(),
            " ? ".bold().cyan(),
            "Help ".into(),
        ])
        .alignment(Alignment::Right);
        frame.render_widget(Paragraph::new(help_hint), main_layout[3]);
    }
}
