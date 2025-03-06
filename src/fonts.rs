use std::{cmp, fs};
use std::{fs::File, io::Write};

use anyhow::{Context, Ok, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::env::Env;
use crate::utils::CreateDirIfNotExists;
use crate::utils::load_project_dirs;
use inquire::MultiSelect;
use zip_extract::extract;

#[derive(Serialize, Deserialize, Clone)]
pub struct FontInfo {
    #[serde(alias = "unpatchedName")]
    unpatched_name: String,
    #[serde(alias = "licenseId")]
    license_id: String,
    #[serde(alias = "RFN")]
    rfn: bool,
    version: String,
    #[serde(alias = "patchedName")]
    patched_name: String,
    #[serde(alias = "folderName")]
    pub folder_name: String,
    #[serde(alias = "imagePreviewFont")]
    image_preview_font: String,
    #[serde(alias = "imagePreviewFontSource")]
    image_preview_font_source: String,
    #[serde(alias = "caskName")]
    cask_name: String,
    #[serde(alias = "repoRelease")]
    repo_release: bool,
    #[serde(alias = "isMonospaced")]
    is_monospaced: bool,
    description: String,
}

#[derive(Serialize, Deserialize)]
pub struct FontRepo {
    pub fonts: Vec<FontInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct NerdFontsInfo {
    pub current_version: String,
}

#[derive(Serialize, Deserialize)]
pub struct FontDownloadStatus {
    pub downloaded: Vec<String>,
    pub installed: Vec<String>,
}

impl FontDownloadStatus {
    pub fn load() -> Result<Self> {
        let dirs = load_project_dirs()?;
        let data_dir = dirs.data_dir();
        let font_zip_dir = data_dir.join("fonts");
        let font_status_path = font_zip_dir.join("font-status.json");
        if !font_status_path.exists() {
            return Ok(FontDownloadStatus {
                downloaded: vec![],
                installed: vec![],
            });
        }
        let font_status: FontDownloadStatus =
            serde_json::from_reader(File::open(font_status_path)?)?;

        Ok(font_status)
    }
    fn save(&self) -> Result<()> {
        let dirs = load_project_dirs()?;
        let data_dir = dirs.data_dir();
        let font_zip_dir = data_dir.join("fonts");
        let font_status_path = font_zip_dir.join("font-status.json");

        let file = match font_status_path.exists() {
            true => File::options().write(true).open(&font_status_path),
            false => File::create(&font_status_path),
        }?;

        serde_json::to_writer(file, self)?;

        Ok(())
    }

    pub fn set_completed(&mut self, font: &str) -> Result<()> {
        if !self.downloaded.contains(&font.into()) {
            self.downloaded.push(font.into());
        }

        self.save()?;

        Ok(())
    }
    pub fn set_installed(&mut self, font: &str) -> Result<()> {
        if !self.installed.contains(&font.into()) {
            self.installed.push(font.into());
        }

        self.save()?;

        Ok(())
    }

    // pub fn remove_completed(&mut self, font: &str) -> Result<()> {
    //     let new_list: Vec<String> = self
    //         .downloaded
    //         .iter()
    //         .filter(|x| !x.eq(&font))
    //         .map(|x| x.clone())
    //         .collect();
    //
    //     self.downloaded = new_list;
    //     self.save()?;
    //
    //     Ok(())
    // }
}

impl FontRepo {
    pub async fn fetch(url: &str) -> Result<Self> {
        let fonts_raw = reqwest::get(url).await?.text().await?;
        let fonts: FontRepo = serde_json::from_str(&fonts_raw)?;

        Ok(fonts)
    }
}

impl NerdFontsInfo {
    pub async fn fetch(url: &str) -> Result<Self> {
        let info_raw = reqwest::get(url).await?.text().await?;
        let res: Self = serde_yaml::from_str(&info_raw)?;

        Ok(res)
    }
}

pub struct FontDownloader;

impl FontDownloader {
    pub async fn download(
        client: &Client,
        font_name: &str,
        version: &str,
        destination_path: &str,
    ) -> Result<()> {
        let url = format!(
            "https://github.com/ryanoasis/nerd-fonts/releases/download/v{version}/{font_name}.zip"
        );
        let res = client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to GET font '{}'", font_name))?;

        let total_size = res
            .content_length()
            .context(format!("Failed to get Content Length for font {font_name}"))?;

        let pb = ProgressBar::new(total_size);

        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
                )?
                .progress_chars("#>-")
        );

        pb.set_message(format!("Downloading {}", font_name));

        let mut file = File::create(destination_path)
            .context(format!("failed to create file for font '{font_name}'"))?;
        let mut downloaded = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.context(format!("Error while downloading font {font_name}"))?;

            file.write_all(&chunk)
                .context(format!("Error while saving font {font_name}"))?;

            let new = cmp::min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }

        pb.finish_with_message(format!("Downloaded {font_name} to {destination_path}"));

        Ok(())
    }
}

pub trait ToFontInfoVec {
    fn to_font_info_vec(&self, font_infos: &Vec<FontInfo>) -> Vec<FontInfo>;
}
pub trait ToStringVec {
    fn to_string_vec(&self) -> Vec<String>;
}

impl ToStringVec for Vec<FontInfo> {
    fn to_string_vec(&self) -> Vec<String> {
        self.iter().map(|it| it.folder_name.clone()).collect()
    }
}

impl ToFontInfoVec for Vec<String> {
    fn to_font_info_vec(&self, font_infos: &Vec<FontInfo>) -> Vec<FontInfo> {
        font_infos
            .iter()
            .filter(|&font_info| self.contains(&font_info.folder_name.clone()))
            .map(|item| item.clone())
            .collect()
    }
}

pub async fn install_font_tui() -> Result<()> {
    let env = Env::prepare()?;
    let mut font_status = FontDownloadStatus::load()?;

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

    let installed_index = fonts
        .iter()
        .enumerate()
        .filter_map(|(index, fontinfo)| {
            match font_status.installed.contains(&fontinfo.folder_name) {
                true => Some(index),
                false => None,
            }
        })
        .collect::<Vec<_>>();
    let client = reqwest::Client::new();
    let selected_fonts = MultiSelect::new("Select Fonts to install:", fonts.to_string_vec())
        .with_vim_mode(true)
        .with_default(&installed_index)
        .prompt()?
        .to_font_info_vec(&fonts);
    let data_dir = dirs.data_dir();
    let font_zip_dir = data_dir.join("fonts");

    font_zip_dir.create_if_not_exists()?;

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
        if font_status.installed.contains(&font.folder_name) {
            continue;
        }
        let zip_file_path = font_zip_dir.join(&font.folder_name).with_extension("zip");
        let zip_file = fs::File::open(zip_file_path)?;
        let font_extract_folder = font_dir.join(&font.folder_name);
        font_extract_folder.create_if_not_exists()?;
        extract(zip_file, &font_extract_folder, true)?;
        font_status.set_installed(&font.folder_name)?;
    }
    Ok(())
}
