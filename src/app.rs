use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use thiserror::Error;
use tokio::{select, sync::mpsc};
use tracing::instrument;

use crate::{
    clock::{Clock, ClockState, Task, error::ClockError},
    help::HelpPane,
    input::TaskInput,
};

type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, PartialEq, Eq)]
pub enum FrontPane {
    Main,
    AddTask,
    Help,
}

#[derive(Debug)]
pub enum AppAction {
    UpdateClockProgress {
        seconds_left: f64,
    },
    UpdateTaskList {
        current_id: Option<usize>,
        tasks: Vec<Task>,
    },
    ClockTimerFinish,
    ClockTimerPauseToggle(bool),
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
        let clock = Arc::new(Clock::new(action_tx));
        Self {
            is_running: true,
            clock,
            clock_state: ClockState::default(),
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
                Some(action) = self.action_rx.recv() => self.handle_action(action),
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
                        clock.reset().await?;
                        tokio::spawn(async move {
                            let _ = clock.run_next_task().await;
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
                    KeyCode::Char('k') => self.clock.kill_current_task().await?,
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

    #[instrument(skip(self))]
    fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::UpdateClockProgress { seconds_left } => {
                self.clock_state.seconds_left = Some(seconds_left);
            }
            AppAction::UpdateTaskList { current_id, tasks } => {
                self.clock_state.current_task_id = current_id;
                self.clock_state.tasks = tasks;
            }
            AppAction::ClockTimerFinish => self.clock_state.seconds_left = None,
            AppAction::ClockTimerPauseToggle(is_paused) => self.clock_state.is_paused = is_paused,
        };
    }

    #[instrument(skip(self, frame))]
    fn draw(&mut self, frame: &mut Frame) {
        self.clock
            .render_with_state(frame, frame.area(), &self.clock_state);

        match self.front_pane {
            FrontPane::Main => {}
            FrontPane::AddTask => self
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
