use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode};
use futures::{FutureExt, StreamExt};
use ratatui::{DefaultTerminal, Frame};
use thiserror::Error;
use tokio::{select, sync::mpsc};

use crate::{
    clock::{Clock, ClockState, Task},
    input::{TaskInput, centered_rect},
};

type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    AddTask,
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
}

#[derive(Debug)]
pub struct App {
    is_running: bool,

    clock: Arc<Clock>,
    clock_state: ClockState,

    input_mode: InputMode,
    task_input: TaskInput,

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
            input_mode: InputMode::Normal,
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
                Some(action) = self.action_rx.recv() => self.handle_action(action, terminal)?,
            };
        }
        Ok(())
    }

    async fn handle_event(&mut self, evt: Event) -> Result<()> {
        match evt {
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Key(key_evt) => match self.input_mode {
                InputMode::Normal => match key_evt.code {
                    KeyCode::Char('q') => self.is_running = false,
                    KeyCode::Char('r') => {
                        let clock = self.clock.clone();
                        tokio::spawn(async move {
                            let _ = clock.run_next_task().await;
                        });
                    }
                    KeyCode::Char('a') => self.input_mode = InputMode::AddTask,
                    _ => {}
                },
                InputMode::AddTask => match key_evt.code {
                    KeyCode::Enter => {
                        if let Ok(task) = self.task_input.get_task() {
                            self.clock.add_task(task).await;
                            self.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Tab => self.task_input.switch_focus(),
                    KeyCode::Esc => self.input_mode = InputMode::Normal,
                    _ => self.task_input.handle_event(evt),
                },
            },
            Event::Mouse(mouse_event) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(_, _) => todo!(),
        }
        Ok(())
    }

    fn handle_action(&mut self, action: AppAction, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            AppAction::UpdateClockProgress { seconds_left } => {
                self.clock_state.seconds_left = seconds_left;

                terminal
                    .draw(|frame| self.draw(frame))
                    .map_err(|_| AppError::DrawFail)?
            }
            AppAction::UpdateTaskList { current_id, tasks } => {
                self.clock_state.current_task_id = current_id;
                self.clock_state.tasks = tasks;
                terminal
                    .draw(|frame| self.draw(frame))
                    .map_err(|_| AppError::DrawFail)?
            }
        };
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.clock
            .render_with_state(frame, frame.area(), &self.clock_state);

        if self.input_mode == InputMode::AddTask {
            self.task_input
                .render(frame, centered_rect(60, 20, frame.area()));
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("fail to draw app")]
    DrawFail,
}
