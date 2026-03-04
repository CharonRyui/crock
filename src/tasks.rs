use std::sync::Arc;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, List, ListItem},
};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc};

use crate::{
    app::{AppAction, TaskPaneAppAction},
    utils::format_time,
};

type Result<T> = std::result::Result<T, TasksError>;

#[derive(Debug, Clone)]
pub struct Task {
    pub content: Arc<str>,
    pub seconds: f64,
}

#[derive(Debug)]
pub struct TaskPaneState {
    pub tasks: Vec<Task>,
    pub current_task_idx: Option<usize>,
    pub focused_task_idx: Option<usize>,
}

#[derive(Debug)]
pub struct TaskPane {
    tasks: Mutex<Vec<Task>>,
    current_task_idx: Mutex<Option<usize>>,
    focused_task_idx: Mutex<Option<usize>>,
    app_action_tx: mpsc::Sender<AppAction>,
}

impl TaskPane {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>, tasks: Vec<Task>) -> (Self, TaskPaneState) {
        (
            Self {
                tasks: Mutex::new(tasks.clone()),
                current_task_idx: Mutex::default(),
                focused_task_idx: Mutex::default(),
                app_action_tx,
            },
            TaskPaneState {
                tasks,
                current_task_idx: None,
                focused_task_idx: None,
            },
        )
    }

    pub async fn replace_focused_task(&self, task: Task) -> Result<()> {
        let focused_idx = self.focused_task_idx.lock().await;
        let mut tasks = self.tasks.lock().await;
        let mut current_task_idx = self.current_task_idx.lock().await;

        if *current_task_idx == *focused_idx {
            *current_task_idx = None;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                    None,
                )))
                .await?;
        }

        if let Some(focused_idx) = *focused_idx {
            tasks[focused_idx] = task;
            self.app_action_tx
                .send(AppAction::UpdateTaskList {
                    current_task_idx: *self.current_task_idx.lock().await,
                    tasks: tasks.clone(),
                    focused_task_idx: Some(focused_idx),
                })
                .await?;
        }
        Ok(())
    }

    pub async fn focus_on_next(&self, offset: isize) -> Result<()> {
        let mut focused_idx = self.focused_task_idx.lock().await;
        let tasks = self.tasks.lock().await;
        if tasks.is_empty() {
            *focused_idx = None;
        } else {
            let next_idx = match *focused_idx {
                Some(idx) => (idx as isize + offset).rem_euclid(tasks.len() as isize) as usize,
                None => 0,
            };
            *focused_idx = Some(next_idx);
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateFocusedTask(
                    *focused_idx,
                )))
                .await?;
        }
        Ok(())
    }

    pub async fn delete_focused_task(&self) -> Result<()> {
        let mut focused_idx_guard = self.focused_task_idx.lock().await;
        let mut tasks = self.tasks.lock().await;
        let mut current_task_idx_guard = self.current_task_idx.lock().await;

        if *current_task_idx_guard == *focused_idx_guard {
            *current_task_idx_guard = None;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                    None,
                )))
                .await?;
        }

        if let Some(focused_idx) = *focused_idx_guard {
            tasks.remove(focused_idx);
            let new_focused_idx = if tasks.is_empty() {
                None
            } else {
                Some(focused_idx.min(tasks.len() - 1))
            };
            *focused_idx_guard = new_focused_idx;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateFocusedTask(
                    new_focused_idx,
                )))
                .await?;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateTasks(
                    tasks.clone(),
                )))
                .await?;
        }

        Ok(())
    }

    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &TaskPaneState) {
        let items: Vec<_> = state
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let mut style = if Some(i) == state.focused_task_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::Gray)
                };

                if Some(i) == state.current_task_idx {
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

        frame.render_widget(list, area);
    }
}

#[derive(Debug, Error)]
pub enum TasksError {
    #[error("fail to send app action")]
    AppActionSendFail(#[from] mpsc::error::SendError<AppAction>),
}
