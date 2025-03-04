mod env;
mod fonts;
mod utils;
use std::fs;

use crate::env::Env;
use crate::utils::CreateDirIfNotExists;
use anyhow::{Context, Ok, Result};
use fonts::{FontDownloadStatus, FontDownloader, ToFontInfoVec, ToStringVec};
use inquire::MultiSelect;
use tokio;
use utils::load_project_dirs;
use zip_extract::extract;

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
    let env = Env::prepare()?;

    let dirs = load_project_dirs()?;
    let font_dir = dirs_sys::home_dir()
        .context("Couldn't get home directory")?
        .join(".local")
        .join("share")
        .join("fonts")
        .join(&env.font.fonts_dir_name);

    font_dir.create_if_not_exists()?;

    let nerd_fonts_current_version = env.font.fetch_root().await?.current_version;
    let fonts = env.font.fetch_fonts().await?;

    let client = reqwest::Client::new();
    let selected_fonts = MultiSelect::new("Select installed fonts:", fonts.to_string_vec())
        .with_vim_mode(true)
        .prompt()?
        .to_font_info_vec(&fonts);
    let data_dir = dirs.data_dir();
    let font_zip_dir = data_dir.join("fonts");

    font_zip_dir.create_if_not_exists()?;

    let mut font_status = FontDownloadStatus::load()?;
    for font in selected_fonts.iter() {
        if font_status.downloaded.contains(&font.folder_name) {
            continue;
        }
        FontDownloader::download(
            &client,
            &font.folder_name,
            &nerd_fonts_current_version,
            font_zip_dir
                .join(&font.folder_name)
                .with_extension("zip")
                .to_str()
                .context(format!("Couldn't save font {}", font.folder_name))?,
        )
        .await?;
        font_status.set_completed(&font.folder_name)?;
    }

    for font in selected_fonts.iter() {
        let zip_file_path = font_zip_dir.join(&font.folder_name).with_extension("zip");
        let zip_file = fs::File::open(zip_file_path)?;
        let font_extract_folder = font_dir.join(&font.folder_name);
        font_extract_folder.create_if_not_exists()?;
        extract(zip_file, &font_extract_folder, true)?;
    }
    Ok(())
}
