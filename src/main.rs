use crock::{application, logger::init_tracing};

fn main() -> color_eyre::Result<()> {
    let _guard = init_tracing();
    color_eyre::install()?;
    ratatui::run(application)?;
    Ok(())
}
