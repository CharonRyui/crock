use thiserror::Error;
use tokio::sync::mpsc;

use crate::app::AppAction;

#[derive(Debug, Error)]
pub enum ClockError {
    #[error("task list is empty")]
    NoTask,
    #[error("timer error: {0}")]
    TimerError(#[from] TimerError),
    #[error("send app action error: {0}")]
    SendActionFail(#[from] mpsc::error::SendError<AppAction>),
}

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer is still running")]
    StillRunning,
}
