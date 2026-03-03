use ratatui::DefaultTerminal;

use crate::app::{App, AppError};

pub mod app;
pub mod clock;
pub mod help;
pub mod input;
pub mod logger;

pub fn application(terminal: &mut DefaultTerminal) -> Result<(), AppError> {
    let mut app = App::default();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("fail to setup tokio runtime");
    rt.block_on(app.run(terminal))
}
