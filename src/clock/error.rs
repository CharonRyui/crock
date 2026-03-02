use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer is still running")]
    StillRunning,
}
