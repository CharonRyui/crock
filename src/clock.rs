use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::Span,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use tokio::sync::{Mutex, mpsc};
use tracing::instrument;
use tui_big_text::{BigText, PixelSize};

use crate::{
    app::AppAction,
    clock::{error::ClockError, timer::Timer},
    utils::format_time,
};

pub mod error;
pub mod timer;

type Result<T> = std::result::Result<T, error::ClockError>;

#[derive(Debug, Default)]
pub struct ClockState {
    pub tasks: Vec<Task>,
    pub current_running_task: Option<usize>,
    pub seconds_left: Option<f64>,
    pub is_paused: bool,
    pub focused_task: Option<usize>,
}

#[derive(Debug)]
pub struct Clock {
    timer: Arc<Timer>,
    tasks: Mutex<Vec<Task>>,
    current_task_idx: Mutex<Option<usize>>,
    app_action_tx: mpsc::Sender<AppAction>,
    focused_task_idx: Mutex<Option<usize>>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub content: Arc<str>,
    pub seconds: f64,
}

impl Clock {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>, tasks: Vec<Task>) -> (Self, ClockState) {
        (
            Self {
                timer: Arc::default(),
                tasks: Mutex::new(tasks.clone()),
                current_task_idx: Mutex::default(),
                focused_task_idx: Mutex::default(),
                app_action_tx,
            },
            ClockState {
                tasks,
                is_paused: true,
                ..Default::default()
            },
        )
    }

    #[instrument(skip(self))]
    pub async fn current_task(&self) -> Option<Task> {
        let task_id = self.current_task_idx.lock().await;
        let tasks = self.tasks.lock().await;
        tasks.get((*task_id)?).cloned()
    }

    #[instrument(skip(self))]
    pub async fn run_next_task(&self) -> Result<()> {
        let app_tx = self.app_action_tx.clone();
        {
            let mut task_id = self.current_task_idx.lock().await;
            if task_id.is_none() {
                *task_id = Some(0)
            }
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_task_idx: *task_id,
                    tasks: self.tasks.lock().await.clone(),
                    focused_task_idx: *self.focused_task_idx.lock().await,
                })
                .await?;
        }

        let current_task = self.current_task().await.ok_or(ClockError::NoTask)?;

        let on_tick = move |seconds_left| {
            let app_tx = app_tx.clone();
            tokio::spawn(async move {
                let _ = app_tx
                    .send(AppAction::UpdateClockProgress { seconds_left })
                    .await;
            });
        };

        self.app_action_tx
            .send(AppAction::UpdateClockProgress {
                seconds_left: current_task.seconds,
            })
            .await?;
        self.app_action_tx
            .send(AppAction::ClockTimerPauseToggle { is_paused: false })
            .await?;
        let timer = self.timer.clone();
        timer.run(current_task.seconds, on_tick).await?;

        let mut task_id = self.current_task_idx.lock().await;

        let tasks = self.tasks.lock().await;
        *task_id = task_id.map(|id| (id + 1) % tasks.len());
        let mut focused_task_idx = self.focused_task_idx.lock().await;
        *focused_task_idx = *task_id;
        self.app_action_tx
            .send(AppAction::UpdateTaskList {
                current_task_idx: *task_id,
                tasks: tasks.clone(),
                focused_task_idx: *focused_task_idx,
            })
            .await?;
        self.app_action_tx
            .send(AppAction::ClockTimerFinish {
                task: Some(current_task),
            })
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn add_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let mut focused_task_idx = self.focused_task_idx.lock().await;
        *focused_task_idx = Some(focused_task_idx.map(|i| i + 1).unwrap_or(0));
        tasks.insert((*focused_task_idx).unwrap(), task);
        self.app_action_tx
            .send(AppAction::UpdateTaskList {
                current_task_idx: *self.current_task_idx.lock().await,
                tasks: tasks.clone(),
                focused_task_idx: *focused_task_idx,
            })
            .await?;
        Ok(())
    }

    pub async fn run_focused(&self) -> Result<()> {
        {
            self.kill_current_task().await?;
            let mut running_task_id = self.current_task_idx.lock().await;
            let focused_task_idx = self.focused_task_idx.lock().await;
            *running_task_id = *focused_task_idx;
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_task_idx: *running_task_id,
                    tasks: self.tasks.lock().await.clone(),
                    focused_task_idx: *focused_task_idx,
                })
                .await?;
        }
        self.run_next_task().await?;
        Ok(())
    }

    pub async fn toggle_pause(&self) -> Result<()> {
        if self.timer.is_running().await {
            self.timer.pause_run().await;
            self.app_action_tx
                .send(AppAction::ClockTimerPauseToggle { is_paused: true })
                .await?;
        } else {
            self.timer.continue_run().await;
            self.app_action_tx
                .send(AppAction::ClockTimerPauseToggle { is_paused: false })
                .await?;
        }
        Ok(())
    }

    pub async fn kill_current_task(&self) -> Result<()> {
        self.timer.stop_run().await;
        self.app_action_tx
            .send(AppAction::ClockTimerFinish { task: None })
            .await?;
        Ok(())
    }

    pub async fn focus_next(&self, offset: isize) -> Result<()> {
        let mut focused_task_idx = self.focused_task_idx.lock().await;
        let tasks = self.tasks.lock().await;
        let current_task_idx = self.current_task_idx.lock().await;
        let idx = if focused_task_idx.is_none() {
            0
        } else {
            (focused_task_idx.unwrap() as isize + offset) as usize % tasks.len()
        };
        *focused_task_idx = Some(idx);
        self.app_action_tx
            .send(AppAction::UpdateTaskList {
                current_task_idx: *current_task_idx,
                tasks: tasks.clone(),
                focused_task_idx: *focused_task_idx,
            })
            .await?;
        Ok(())
    }

    pub async fn delete_focused_task(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let mut focused_task_idx = self.focused_task_idx.lock().await;
        let mut current_task_idx = self.current_task_idx.lock().await;
        if let Some(focused_idx) = *focused_task_idx {
            tasks.remove(focused_idx);
            if Some(focused_idx) == *current_task_idx {
                *current_task_idx = None;
                self.kill_current_task().await?;
                self.app_action_tx
                    .send(AppAction::ClockTimerFinish { task: None })
                    .await?;
            }
            if focused_idx >= tasks.len() {
                *focused_task_idx = Some(tasks.len().saturating_sub(1));
            }
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_task_idx: *current_task_idx,
                    tasks: tasks.clone(),
                    focused_task_idx: *focused_task_idx,
                })
                .await?;
        }
        Ok(())
    }

    pub async fn replace_focused_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let focused_task_idx = self.focused_task_idx.lock().await;
        let current_task_idx = self.current_task_idx.lock().await;
        if let Some(focused_idx) = *focused_task_idx {
            tasks[focused_idx] = task;
            if Some(focused_idx) == *current_task_idx {
                self.kill_current_task().await?;
            }
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_task_idx: *current_task_idx,
                    tasks: tasks.clone(),
                    focused_task_idx: *focused_task_idx,
                })
                .await?;
        }
        Ok(())
    }

    #[instrument(skip(self, frame, area))]
    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &ClockState) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        if state.tasks.is_empty() {
            let paragraph = Paragraph::new("Press 'a' to add item")
                .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(paragraph, layout[0]);
        } else if let Some(task_id) = state.current_running_task {
            let gauge_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(layout[0]);
            let task = &state.tasks[task_id];
            if let Some(seconds_left) = state.seconds_left {
                let ration = if task.seconds > 0.0 {
                    (seconds_left / task.seconds).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let label = Span::style(
                    format!("{} left", format_time(seconds_left)).into(),
                    Style::default(),
                );
                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::BOTTOM))
                    .gauge_style(
                        Style::default()
                            .fg(if state.is_paused {
                                Color::Red
                            } else {
                                Color::Cyan
                            })
                            .bg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .ratio(ration)
                    .label(label);
                let big_text = BigText::builder()
                    .pixel_size(PixelSize::Full)
                    .style(Style::new().blue())
                    .lines(vec![task.content.blue().into()])
                    .centered()
                    .build();
                frame.render_widget(big_text, gauge_layout[0]);
                frame.render_widget(gauge, gauge_layout[1]);
            } else {
                let big_text_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(layout[0]);
                let content_text = BigText::builder()
                    .pixel_size(PixelSize::Full)
                    .style(Style::new().blue())
                    .lines(vec![task.content.blue().into()])
                    .centered()
                    .build();
                let time_text = BigText::builder()
                    .pixel_size(PixelSize::Quadrant)
                    .style(Style::new().cyan())
                    .lines(vec![format_time(task.seconds).white().into()])
                    .centered()
                    .build();
                frame.render_widget(content_text, big_text_layout[0]);
                frame.render_widget(time_text, big_text_layout[1]);
            }
        } else {
            let paragraph = Paragraph::new("Press 'r' to start or 'c' to continue")
                .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(paragraph, layout[0]);
        }

        let items: Vec<_> = state
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let mut style = if Some(i) == state.focused_task {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::Gray)
                };

                if Some(i) == state.current_running_task {
                    style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                }
                let content = format!(
                    " [{}] {} ({})",
                    i + 1,
                    task.content,
                    format_time(task.seconds)
                );
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
