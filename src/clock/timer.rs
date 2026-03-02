use std::time::Duration;

use tokio::sync::Mutex;

use crate::clock::error::TimerError;

type Result<T> = std::result::Result<T, TimerError>;

#[derive(Debug, Default)]
pub struct Timer {
    left_seconds: Mutex<i64>,
}

impl Timer {
    pub async fn start<T: Fn(i64) + Send + 'static, F: FnOnce() + Send + 'static>(
        &self,
        seconds: i64,
        on_tick: T,
        on_finish: F,
    ) -> Result<()> {
        {
            let mut left_seconds = self.left_seconds.lock().await;
            if *left_seconds > 0 {
                return Err(TimerError::StillRunning);
            }
            *left_seconds = seconds;
        }
        loop {
            let mut left_seconds = self.left_seconds.lock().await;
            if *left_seconds <= 0 {
                on_finish();
                break;
            }
            *left_seconds -= 1;
            on_tick(*left_seconds);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn stop(&self) {
        let mut left_seconds = self.left_seconds.lock().await;
        *left_seconds = 0;
    }
}
