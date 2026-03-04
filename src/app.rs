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
    clock::{Clock, ClockState, Task, error::ClockError},
    config::get_config_tasks,
    help::HelpPane,
    input::TaskInput,
    utils::format_time,
};

type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, PartialEq, Eq)]
pub enum FrontPane {
    Main,
    AddTask,
    EditTask,
    Help,
}

#[derive(Debug)]
pub enum AppAction {
    UpdateClockProgress {
        seconds_left: f64,
    },
    UpdateTaskList {
        current_task_idx: Option<usize>,
        tasks: Vec<Task>,
        focused_task_idx: Option<usize>,
    },
    ClockTimerFinish {
        task: Option<Task>,
    },
    ClockTimerPauseToggle {
        is_paused: bool,
    },
}

#[derive(Debug)]
pub struct App {
    is_running: bool,

    clock: Arc<Clock>,
    clock_state: ClockState,

    front_pane: FrontPane,

    task_input: TaskInput,
    help_pane: HelpPane,

    action_rx: mpsc::Receiver<AppAction>,
}

impl Default for App {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(128);
        let preset_tasks = get_config_tasks().clone();
        let (clock, clock_state) = Clock::new(action_tx, preset_tasks);
        Self {
            is_running: true,
            clock: Arc::new(clock),
            clock_state,
            action_rx,
            help_pane: HelpPane,
            front_pane: FrontPane::Main,
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
                FrontPane::Main => match key_evt.code {
                    KeyCode::Char('q') => self.is_running = false,
                    KeyCode::Char('r') => {
                        let clock = self.clock.clone();
                        tokio::spawn(async move {
                            let _ = clock.run_focused().await;
                        });
                    }
                    KeyCode::Char('c') => {
                        let clock = self.clock.clone();
                        tokio::spawn(async move {
                            let _ = clock.run_next_task().await;
                        });
                    }
                    KeyCode::Char('a') => self.front_pane = FrontPane::AddTask,
                    KeyCode::Char('?') => self.front_pane = FrontPane::Help,
                    KeyCode::Char('p') => self.clock.toggle_pause().await?,
                    KeyCode::Char('t') => self.clock.kill_current_task().await?,
                    KeyCode::Char('j') => self.clock.focus_next(1).await?,
                    KeyCode::Char('k') => self.clock.focus_next(-1).await?,
                    KeyCode::Char('d') => self.clock.delete_focused_task().await?,
                    KeyCode::Char('e') => {
                        if self.clock_state.focused_task.is_some() {
                            self.front_pane = FrontPane::EditTask
                        }
                    }
                    _ => {}
                },
                FrontPane::AddTask => match key_evt.code {
                    KeyCode::Enter => {
                        if let Ok(task) = self.task_input.get_task() {
                            self.clock.add_task(task).await?;
                            self.front_pane = FrontPane::Main;
                        }
                    }
                    KeyCode::Tab => self.task_input.switch_focus(),
                    KeyCode::Esc => self.front_pane = FrontPane::Main,
                    _ => self.task_input.handle_event(evt),
                },
                FrontPane::EditTask => match key_evt.code {
                    KeyCode::Enter => {
                        if let Ok(task) = self.task_input.get_task() {
                            self.clock.replace_focused_task(task).await?;
                            self.front_pane = FrontPane::Main;
                        }
                    }
                    KeyCode::Tab => self.task_input.switch_focus(),
                    KeyCode::Esc => self.front_pane = FrontPane::Main,
                    _ => self.task_input.handle_event(evt),
                },
                FrontPane::Help => match key_evt.code {
                    KeyCode::Esc | KeyCode::Char('?') => self.front_pane = FrontPane::Main,
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
            AppAction::UpdateClockProgress { seconds_left } => {
                self.clock_state.seconds_left = Some(seconds_left);
            }
            AppAction::UpdateTaskList {
                current_task_idx,
                tasks,
                focused_task_idx,
            } => {
                self.clock_state.current_running_task = current_task_idx;
                self.clock_state.tasks = tasks;
                self.clock_state.focused_task = focused_task_idx;
            }
            AppAction::ClockTimerFinish { task } => {
                if let Some(task) = task {
                    Notification::new()
                    .summary("Task finished, go next?")
                    .body(&format!(
                        "Task: {} is finished, took {}. Go back to the app and press 'c' to start next task.",
                        task.content,
                        format_time(task.seconds)
                    ))
                    .appname("Crock")
                    .icon("alarm-clock")
                    .timeout(0)
                    .show_async().await?;
                }
                self.clock_state.seconds_left = None
            }
            AppAction::ClockTimerPauseToggle { is_paused } => {
                self.clock_state.is_paused = is_paused
            }
        };
        Ok(())
    }

    #[instrument(skip(self, frame))]
    fn draw(&mut self, frame: &mut Frame) {
        self.clock
            .render_with_state(frame, frame.area(), &self.clock_state);

        match self.front_pane {
            FrontPane::Main => {}
            FrontPane::AddTask | FrontPane::EditTask => self
                .task_input
                .render(frame, centered_rect(60, 20, frame.area())),
            FrontPane::Help => self
                .help_pane
                .render(frame, centered_rect(60, 60, frame.area())),
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("fail to draw app")]
    DrawFail,
    #[error("clock error: {0}")]
    ClockError(#[from] ClockError),
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
