use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph},
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
            .send(AppAction::Clock(ClockAppAction::TimerFinished))
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
            .send(AppAction::Clock(ClockAppAction::TimerFinished))
            .await?;
        Ok(())
    }

    #[instrument(skip(self, frame, area))]
    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &ClockState) {
        // 顶部 70% 用于显示当前倒计时和任务，底部 30% 用于显示“下一项”预览
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // --- 1. 渲染当前任务 (Current Task Section) ---
        if let Some(task) = &state.current_task {
            let current_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(main_layout[0]);

            // 任务名称（大字）
            let task_name = BigText::builder()
                .pixel_size(PixelSize::Full)
                .style(Style::new().blue().bold())
                .lines(vec![task.content.to_string().into()])
                .centered()
                .build();
            frame.render_widget(task_name, current_layout[0]);

            // 倒计时进度条
            if let Some(seconds_left) = state.seconds_left {
                let ratio = if task.seconds > 0.0 {
                    (seconds_left / task.seconds).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let color = if state.is_paused {
                    Color::Red
                } else {
                    Color::Cyan
                };
                let label = format!("{} left", format_time(seconds_left));

                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(" Progress "))
                    .gauge_style(
                        Style::default()
                            .fg(color)
                            .bg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .ratio(ratio)
                    .label(Span::styled(label, Style::default().fg(Color::White)));

                frame.render_widget(gauge, current_layout[1]);
            }
        } else {
            // 无任务状态
            let welcome = Paragraph::new("No active task.\nPress 's' to start or 'a' to add.")
                .alignment(Alignment::Center)
                .block(Block::bordered());
            frame.render_widget(welcome, main_layout[0]);
        }

        // --- 2. 渲染下一项任务 (Next Task Section) ---
        let next_block = Block::bordered()
            .title(" Next Up ")
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::DarkGray));

        if let Some(next) = &state.next_task {
            let next_text = format!(
                "Coming up: {} ({})",
                next.content,
                format_time(next.seconds)
            );
            let paragraph = Paragraph::new(next_text)
                .block(next_block)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, main_layout[1]);
        } else {
            let paragraph = Paragraph::new("Queue is empty")
                .block(next_block)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, main_layout[1]);
        }
    }
}
