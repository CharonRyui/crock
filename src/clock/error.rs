use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClockError {
    #[error("task list is empty")]
    NoTask,
    #[error("timer error: {0}")]
    TimerError(#[from] TimerError),
}

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer is still running")]
    StillRunning,
}
