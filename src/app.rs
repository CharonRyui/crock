use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode};
use futures::{FutureExt, StreamExt};
use notify_rust::Notification;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use thiserror::Error;
use tokio::{select, sync::mpsc};
use tracing::instrument;

use crate::{
    clock::{Clock, ClockState},
    config::get_config_tasks,
    help::HelpPane,
    input::TaskInput,
    tasks::{Task, TaskPane, TaskPaneState},
    utils::format_time,
};

type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, PartialEq, Eq)]
pub enum FrontPane {
    Clock,
    AddTask,
    EditTask,
    TaskPane,
    Help,
}

#[derive(Debug)]
pub enum AppAction {
    TaskPane(TaskPaneAppAction),
    Clock(ClockAppAction),
}

#[derive(Debug)]
pub enum TaskPaneAppAction {
    UpdateTasks(Vec<Task>),
    UpdateCurrentTask(Option<usize>),
    UpdateFocusedTask(Option<usize>),
    InterruptCurrentTask,
}

#[derive(Debug)]
pub enum ClockAppAction {
    UpdateSecondsLeft(f64),
    TimerFinished(Option<Task>),
}

#[derive(Debug)]
pub struct App {
    is_running: bool,

    front_pane: FrontPane,

    task_pane: TaskPane,
    task_pane_state: TaskPaneState,

    clock: Arc<Clock>,
    clock_state: ClockState,

    task_input: TaskInput,
    help_pane: HelpPane,

    action_rx: mpsc::Receiver<AppAction>,
}

impl Default for App {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(128);
        let preset_tasks = get_config_tasks().clone();
        let (clock, clock_state) = Clock::new(action_tx.clone());
        let (task_pane, task_pane_state) = TaskPane::new(action_tx, preset_tasks);
        Self {
            is_running: true,
            clock: Arc::new(clock),
            clock_state,
            task_pane_state,
            task_pane,
            action_rx,
            help_pane: HelpPane,
            front_pane: FrontPane::Clock,
            task_input: TaskInput::default(),
        }
    }
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut event_stream = EventStream::new();
        loop {
            if !self.is_running {
                break;
            }
            let event = event_stream.next().fuse();
            terminal
                .draw(|frame| {
                    self.draw(frame);
                })
                .map_err(|_| AppError::DrawFail)?;
            select! {
                maybe_event = event => {
                    if let Some(Ok(e)) = maybe_event {
                        self.handle_event(e).await?;
                    }
                }
                Some(action) = self.action_rx.recv() => self.handle_action(action).await?,
            };
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn handle_event(&mut self, evt: Event) -> Result<()> {
        match evt {
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Key(key_evt) => match self.front_pane {
                FrontPane::Clock => match key_evt.code {
                    KeyCode::Char('q') => self.is_running = false,
                    KeyCode::Char('r') => {
                        if let Some((current_task, next_task)) =
                            self.task_pane.get_current_and_next_tasks_to_run().await
                        {
                            self.clock_state.current_task = Some(current_task.clone());
                            self.clock_state.next_task = Some(next_task);
                            let clock = self.clock.clone();
                            tokio::spawn(async move {
                                let _ = clock.run_task(current_task).await;
                            });
                        }
                    }
                    KeyCode::Char('?') => self.front_pane = FrontPane::Help,
                    KeyCode::Char('p') => {
                        self.clock.toggle_pause().await;
                        self.clock_state.is_paused = !self.clock_state.is_paused
                    }
                    KeyCode::Char('t') => self.clock.kill_current_task().await?,
                    KeyCode::Char('e') => self.front_pane = FrontPane::TaskPane,
                    _ => {}
                },
                FrontPane::AddTask => match key_evt.code {
                    KeyCode::Enter => {
                        if let Ok(task) = self.task_input.get_task() {
                            self.task_pane.insert_task(task).await?;
                            self.front_pane = FrontPane::TaskPane;
                        }
                    }
                    KeyCode::Tab => self.task_input.switch_focus(),
                    KeyCode::Esc => self.front_pane = FrontPane::TaskPane,
                    _ => self.task_input.handle_event(evt),
                },
                FrontPane::EditTask => match key_evt.code {
                    KeyCode::Enter => {
                        if let Ok(task) = self.task_input.get_task() {
                            self.task_pane.replace_focused_task(task).await?;
                            self.front_pane = FrontPane::TaskPane;
                        }
                    }
                    KeyCode::Tab => self.task_input.switch_focus(),
                    KeyCode::Esc => self.front_pane = FrontPane::TaskPane,
                    _ => self.task_input.handle_event(evt),
                },
                FrontPane::Help => match key_evt.code {
                    KeyCode::Esc | KeyCode::Char('?') => self.front_pane = FrontPane::Clock,
                    _ => {}
                },
                FrontPane::TaskPane => match key_evt.code {
                    KeyCode::Char('a') => self.front_pane = FrontPane::AddTask,
                    KeyCode::Char('e') => self.front_pane = FrontPane::EditTask,
                    KeyCode::Char('d') => self.task_pane.delete_focused_task().await?,
                    KeyCode::Char('j') => self.task_pane.focus_on_next(1).await?,
                    KeyCode::Char('k') => self.task_pane.focus_on_next(-1).await?,
                    KeyCode::Char('q') => self.front_pane = FrontPane::Clock,
                    KeyCode::Esc => self.front_pane = FrontPane::Clock,
                    _ => {}
                },
            },
            Event::Mouse(_mouse_evt) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(_, _) => todo!(),
        }
        Ok(())
    }

    #[instrument(skip(self), err)]
    async fn handle_action(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Clock(clock_app_action) => match clock_app_action {
                ClockAppAction::UpdateSecondsLeft(seconds_left) => {
                    self.clock_state.seconds_left = Some(seconds_left)
                }
                ClockAppAction::TimerFinished(task) => {
                    self.clock_state.seconds_left = None;
                    self.clock_state.is_paused = true;
                    if let Some(task) = task {
                        Notification::new()
                            .appname("crock")
                            .body(&format!(
                                "Task: {} is finished, you've been on it for {}",
                                task.content,
                                format_time(task.seconds)
                            ))
                            .icon("alarm-clock")
                            .summary("Crock Task Time Ends")
                            .show_async()
                            .await?;

                        self.task_pane.finish_current_task().await;
                        if let Some((current_task, next_task)) =
                            self.task_pane.get_current_and_next_tasks_to_run().await
                        {
                            self.clock_state.current_task = Some(current_task.clone());
                            self.clock_state.next_task = Some(next_task);
                        } else {
                            self.clock_state.current_task = None;
                            self.clock_state.next_task = None;
                        }
                    }
                }
            },
            AppAction::TaskPane(task_pane_app_action) => match task_pane_app_action {
                TaskPaneAppAction::UpdateTasks(tasks) => {
                    self.task_pane_state.tasks = tasks;
                    if let Some((current_task, next_task)) =
                        self.task_pane.get_current_and_next_tasks_to_run().await
                    {
                        self.clock_state.current_task = Some(current_task);
                        self.clock_state.next_task = Some(next_task);
                    }
                }
                TaskPaneAppAction::InterruptCurrentTask => {
                    self.clock.kill_current_task().await?;
                }
                TaskPaneAppAction::UpdateCurrentTask(task) => {
                    self.task_pane_state.current_task_idx = task
                }
                TaskPaneAppAction::UpdateFocusedTask(task) => {
                    self.task_pane_state.focused_task_idx = task
                }
            },
        };
        Ok(())
    }

    #[instrument(skip(self, frame))]
    fn draw(&mut self, frame: &mut Frame) {
        match self.front_pane {
            FrontPane::Clock => {
                self.clock
                    .render_with_state(frame, frame.area(), &self.clock_state);
            }
            FrontPane::AddTask | FrontPane::EditTask => self
                .task_input
                .render(frame, centered_rect(60, 20, frame.area())),
            FrontPane::Help => self
                .help_pane
                .render(frame, centered_rect(60, 60, frame.area())),
            FrontPane::TaskPane => {
                self.task_pane.render_with_state(
                    frame,
                    centered_rect(60, 60, frame.area()),
                    &self.task_pane_state,
                );
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("fail to draw app")]
    DrawFail,
    #[error("clock error: {0}")]
    ClockError(#[from] crate::clock::error::ClockError),
    #[error("task pane error: {0}")]
    TaskPaneError(#[from] crate::tasks::TasksError),
    #[error("notification error: {0}")]
    NotificationError(#[from] notify_rust::error::Error),
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
