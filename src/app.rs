use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::clock::Clock;

#[derive(Debug)]
pub enum AppAction {
    UpdateClockProgress(f64),
}

#[derive(Debug)]
pub struct App {
    clock: Clock,
    action_rx: mpsc::Receiver<AppAction>,
}

impl Default for App {
    fn default() -> Self {
        let (action_tx, action_rx) = mpsc::channel(128);
        let clock = Clock::new(action_tx);
        Self { clock, action_rx }
    }
}

impl App {
    pub fn run(&self, terminal: &mut DefaultTerminal) {
        loop {}
    }
}
