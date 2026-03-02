use crock::application;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(application)?;
    Ok(())
}
