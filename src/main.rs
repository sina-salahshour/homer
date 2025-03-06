mod env;
mod fonts;
mod utils;

use anyhow::Result;

use fonts::install_font_tui;

/// TODO: Cache Fetched Font repo and info.
/// font fetcher should:
/// show interactive list of fonts with installed ones already checked
/// clean newly unselected fonts
/// fetch new fonts
/// extract them to desired location
///
/// also should have a checkpoint for each step in case of failures (maybe in future implement
/// it?)
#[tokio::main]
async fn main() -> Result<()> {
    install_font_tui().await
}
