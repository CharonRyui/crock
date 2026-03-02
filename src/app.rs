use crossterm::event::{Event, EventStream, KeyCode, KeyEvent};
use futures::{FutureExt, StreamExt};
use ratatui::DefaultTerminal;
use thiserror::Error;
use tokio::{select, sync::mpsc};

use crate::clock::Clock;

type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppAction {
    UpdateClockProgress(f64),
}

#[derive(Debug)]
pub struct App {
    is_running: bool,
    clock: Clock,
    action_rx: mpsc::Receiver<AppAction>,
}

impl Default for App {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(128);
        let clock = Clock::new(action_tx);
        Self {
            is_running: true,
            clock,
            action_rx,
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
                    self.clock
                        .render_with_current_seconds_left(frame, frame.area(), 0.0)
                })
                .map_err(|_| AppError::DrawFail)?;
            select! {
                maybe_event = event => {
                    if let Some(Ok(e)) = maybe_event {
                        self.handle_event(e)?;
                    }
                }
                Some(action) = self.action_rx.recv() => self.handle_action(action, terminal)?,
            };
        }
        Ok(())
    }

    fn handle_event(&mut self, e: Event) -> Result<()> {
        match e {
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Mouse(mouse_event) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(_, _) => todo!(),
        }
        Ok(())
    }

    fn handle_key_event(&mut self, e: KeyEvent) {
        match e.code {
            KeyCode::Char('q') => self.is_running = false,
            KeyCode::Char('r') => self.run_clock(),
            _ => {}
        }
    }

    fn handle_action(&mut self, action: AppAction, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            AppAction::UpdateClockProgress(seconds_left) => terminal
                .draw(|frame| {
                    self.clock
                        .render_with_current_seconds_left(frame, frame.area(), seconds_left)
                })
                .map_err(|_| AppError::DrawFail)?,
        };
        Ok(())
    }

    fn run_clock(&self) {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("fail to draw app")]
    DrawFail,
}
