use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, Paragraph},
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

    pub async fn set_focused_task_current(&self) -> Result<()> {
        let mut current_task_idx = self.current_task_idx.lock().await;
        let focused_task_idx = self.focused_task_idx.lock().await;
        if *current_task_idx != *focused_task_idx {
            *current_task_idx = *focused_task_idx;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                    *current_task_idx,
                )))
                .await?;
        }
        Ok(())
    }

    pub async fn finish_current_task(&self) {
        let mut current_task_idx = self.current_task_idx.lock().await;
        let tasks = self.tasks.lock().await;
        if tasks.is_empty() {
            *current_task_idx = None;
        } else {
            let next_idx = match *current_task_idx {
                Some(idx) => (idx + 1).rem_euclid(tasks.len()),
                None => 0,
            };
            *current_task_idx = Some(next_idx);
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                    *current_task_idx,
                )))
                .await
                .ok();
        }
    }

    pub async fn replace_focused_task(&self, task: Task) -> Result<()> {
        let focused_idx = self.focused_task_idx.lock().await;
        if let Some(focused_idx) = *focused_idx {
            let mut tasks = self.tasks.lock().await;
            let mut current_task_idx = self.current_task_idx.lock().await;

            if *current_task_idx == Some(focused_idx) {
                *current_task_idx = None;
                self.app_action_tx
                    .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                        None,
                    )))
                    .await?;
            }
            tasks[focused_idx] = task;
            self.app_action_tx
                .send(AppAction::TaskPane(TaskPaneAppAction::UpdateTasks(
                    tasks.clone(),
                )))
                .await?;
        }
        Ok(())
    }

    pub async fn insert_task(&self, task: Task) -> Result<()> {
        let mut focused_idx = self.focused_task_idx.lock().await;
        let mut tasks = self.tasks.lock().await;
        let insert_idx = focused_idx.map(|idx| idx + 1).unwrap_or(0);
        tasks.insert(insert_idx, task);
        *focused_idx = Some(insert_idx);
        self.app_action_tx
            .send(AppAction::TaskPane(TaskPaneAppAction::UpdateTasks(
                tasks.clone(),
            )))
            .await?;
        self.app_action_tx
            .send(AppAction::TaskPane(TaskPaneAppAction::UpdateFocusedTask(
                *focused_idx,
            )))
            .await?;
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

    pub async fn get_current_and_next_tasks_to_run(&self) -> Result<Option<(Task, Task)>> {
        let mut current_task_idx = self.current_task_idx.lock().await;
        let tasks = self.tasks.lock().await;
        if tasks.is_empty() {
            Ok(None)
        } else {
            if current_task_idx.is_none() {
                *current_task_idx = Some(0);
                self.app_action_tx
                    .send(AppAction::TaskPane(TaskPaneAppAction::UpdateCurrentTask(
                        *current_task_idx,
                    )))
                    .await?;
            }
            let idx = current_task_idx.unwrap();
            let next_idx = (idx + 1).rem_euclid(tasks.len());
            Ok(Some((tasks[idx].clone(), tasks[next_idx].clone())))
        }
    }

    pub fn render_with_state(&self, frame: &mut Frame, area: Rect, state: &TaskPaneState) {
        frame.render_widget(Clear, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let items: Vec<ListItem> = state
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let is_focused = Some(i) == state.focused_task_idx;
                let is_current = Some(i) == state.current_task_idx;

                let (status_icon, status_style) = if is_current {
                    (" ⏱ ", Style::default().fg(Color::Cyan).bold())
                } else {
                    ("   ", Style::default().fg(Color::DarkGray))
                };

                let content = Line::from(vec![
                    Span::styled(status_icon, status_style),
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(Color::Indexed(240)),
                    ),
                    Span::styled(
                        task.content.to_string(),
                        if is_focused {
                            Style::default().bold()
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        format!(" ({})", format_time(task.seconds)),
                        Style::default().fg(Color::Indexed(243)).italic(),
                    ),
                ]);

                let item = ListItem::new(content);
                if is_focused {
                    item.bg(Color::Indexed(235))
                } else {
                    item
                }
            })
            .collect();

        let list_block = Block::bordered()
            .title(Span::styled(" Task List ", Style::default().cyan().bold()))
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Indexed(240)));

        let list = List::new(items)
            .block(list_block)
            .highlight_symbol("❯ ")
            .highlight_style(Style::default().fg(Color::Yellow).bold());

        frame.render_widget(list, layout[0]);

        let footer = Line::from(vec![
            " a ".bold().cyan(),
            "Add ".into(),
            " r ".bold().cyan(),
            "Edit ".into(),
            " d ".bold().cyan(),
            "Del ".into(),
            " Enter ".bold().cyan(),
            "Run ".into(),
            " Esc ".bold().cyan(),
            "Back ".into(),
        ]);
        frame.render_widget(
            Paragraph::new(footer).alignment(ratatui::layout::Alignment::Center),
            layout[1],
        );
    }
}

#[derive(Debug, Error)]
pub enum TasksError {
    #[error("fail to send app action")]
    AppActionSendFail(#[from] mpsc::error::SendError<AppAction>),
}
