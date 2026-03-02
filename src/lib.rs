use ratatui::DefaultTerminal;

use crate::app::{App, AppError};

pub mod app;
pub mod clock;
pub mod input;

pub fn application(terminal: &mut DefaultTerminal) -> Result<(), AppError> {
    let mut app = App::default();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("fail to setup tokio runtime");
    rt.block_on(app.run(terminal))
}
