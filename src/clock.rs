use tokio::sync::{Mutex, mpsc};

use crate::{
    app::AppAction,
    clock::{error::ClockError, timer::Timer},
};

pub mod error;
pub mod timer;

type Result<T> = std::result::Result<T, error::ClockError>;

#[derive(Debug)]
pub struct Clock {
    timer: Timer,
    pub tasks: Vec<Task>,
    pub current_task_id: Mutex<usize>,
    app_action_tx: mpsc::Sender<AppAction>,
}

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub seconds: i64,
}

impl Clock {
    pub fn new(app_action_tx: mpsc::Sender<AppAction>) -> Self {
        Self {
            timer: Timer::default(),
            tasks: Vec::new(),
            current_task_id: Mutex::new(0),
            app_action_tx,
        }
    }

    pub async fn current_task_seconds(&self) -> Option<i64> {
        self.tasks
            .get(*self.current_task_id.lock().await)
            .map(|task| task.seconds)
    }

    pub async fn start_next_task(&mut self) -> Result<()> {
        let app_tx = self.app_action_tx.clone();
        let on_tick = move |left_seconds| {
            let app_tx = app_tx.clone();
            tokio::spawn(async move {
                let _ = app_tx
                    .send(AppAction::UpdateClockProgress(left_seconds))
                    .await;
            });
        };

        let task_seconds = self
            .current_task_seconds()
            .await
            .ok_or(ClockError::NoTask)?;
        self.timer.start(task_seconds, on_tick).await?;

        let mut task_id = self.current_task_id.lock().await;
        *task_id = (*task_id + 1) % self.tasks.len();
        Ok(())
    }
}
