use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::instrument;

use crate::{
    app::AppAction,
    clock::{error::ClockError, timer::Timer},
};

pub mod error;
pub mod timer;

type Result<T> = std::result::Result<T, error::ClockError>;

#[derive(Debug, Default)]
pub struct ClockState {
    pub tasks: Vec<Task>,
    pub current_task_id: Option<usize>,
    pub seconds_left: Option<f64>,
}

#[derive(Debug)]
pub struct Clock {
    timer: Arc<Timer>,
    tasks: Mutex<Vec<Task>>,
    current_task_id: Mutex<Option<usize>>,
    app_action_tx: mpsc::Sender<AppAction>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub content: Arc<str>,
    pub seconds: f64,
}

impl Clock {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>) -> Self {
        Self {
            timer: Arc::default(),
            tasks: Mutex::default(),
            current_task_id: Mutex::default(),
            app_action_tx,
        }
    }

    #[instrument(skip(self))]
    pub async fn current_task(&self) -> Option<Task> {
        let task_id = self.current_task_id.lock().await;
        let tasks = self.tasks.lock().await;
        tasks.get((*task_id)?).cloned()
    }

    #[instrument(skip(self))]
    pub async fn run_next_task(&self) -> Result<()> {
        let app_tx = self.app_action_tx.clone();
        {
            let mut task_id = self.current_task_id.lock().await;
            if task_id.is_none() {
                *task_id = Some(0)
            }
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_id: *task_id,
                    tasks: self.tasks.lock().await.clone(),
                })
                .await?;
        }

        let task_seconds = self.current_task().await.ok_or(ClockError::NoTask)?.seconds;

        let on_tick = move |seconds_left| {
            let app_tx = app_tx.clone();
            tokio::spawn(async move {
                let _ = app_tx
                    .send(AppAction::UpdateClockProgress { seconds_left })
                    .await;
            });
        };

        let (finish_tx, finish_rx) = oneshot::channel();
        let on_finish = move || {
            let _ = finish_tx.send(true);
        };

        self.app_action_tx
            .send(AppAction::UpdateClockProgress {
                seconds_left: task_seconds,
            })
            .await?;
        let timer = self.timer.clone();
        tokio::spawn(async move {
            let _ = timer.run(task_seconds, on_tick, on_finish).await;
        });

        if Ok(true) == finish_rx.await {
            let mut task_id = self.current_task_id.lock().await;
            let tasks = self.tasks.lock().await;
            *task_id = task_id.map(|id| (id + 1) % tasks.len());
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_id: *task_id,
                    tasks: tasks.clone(),
                })
                .await?;
            self.app_action_tx.send(AppAction::ClockTimerFinish).await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn add_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let task_id = self.current_task_id.lock().await;
        tasks.push(task);
        self.app_action_tx
            .send(AppAction::UpdateTaskList {
                current_id: *task_id,
                tasks: tasks.clone(),
            })
            .await?;
        Ok(())
    }

    pub async fn reset(&self) -> Result<()> {
        let mut task_id = self.current_task_id.lock().await;
        *task_id = None;
        self.app_action_tx
            .send(AppAction::UpdateTaskList {
                current_id: None,
                tasks: self.tasks.lock().await.clone(),
            })
            .await?;
        Ok(())
    }

    #[instrument(skip(self, frame, area))]
    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &ClockState) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        if state.tasks.is_empty() {
            let paragraph = Paragraph::new("Press 'a' to add item")
                .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(paragraph, layout[0]);
        } else if let Some(task_id) = state.current_task_id {
            let task = &state.tasks[task_id];
            if let Some(seconds_left) = state.seconds_left {
                let ration = if task.seconds > 0.0 {
                    (seconds_left / task.seconds).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let label = format!("Running{}\n{}s remaining", task.content, seconds_left);
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
                    Paragraph::new(&*task.content).block(Block::default().borders(Borders::BOTTOM));
                frame.render_widget(paragraph, layout[0]);
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
                let style = if Some(i) == state.current_task_id {
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
