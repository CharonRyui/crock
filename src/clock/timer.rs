use std::time::Duration;

use tokio::sync::Mutex;
use tracing::instrument;

use crate::clock::error::TimerError;

type Result<T> = std::result::Result<T, TimerError>;

#[derive(Debug, Default)]
pub struct Timer {
    left_seconds: Mutex<f64>,
    is_running: Mutex<bool>,
}

impl Timer {
    #[instrument(skip(self, on_tick, on_finish))]
    pub async fn run<T: Fn(f64) + Send + 'static, F: FnOnce() + Send + 'static>(
        &self,
        seconds: f64,
        on_tick: T,
        on_finish: F,
    ) -> Result<()> {
        {
            let mut left_seconds = self.left_seconds.lock().await;
            if *left_seconds > 0.0 {
                return Err(TimerError::StillRunning);
            }
            *left_seconds = seconds;
        }

        self.continue_run().await;
        loop {
            {
                if !*self.is_running.lock().await {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            }

            {
                let mut left_seconds = self.left_seconds.lock().await;
                if *left_seconds <= 0.0 {
                    self.pause_run().await;
                    on_finish();
                    break;
                }
                *left_seconds -= 1.0;
                on_tick(*left_seconds);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn pause_run(&self) {
        *self.is_running.lock().await = false
    }

    pub async fn continue_run(&self) {
        *self.is_running.lock().await = true
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }

    pub async fn stop_run(&self) {
        self.pause_run().await;
        *self.left_seconds.lock().await = 0.0
    }
}
