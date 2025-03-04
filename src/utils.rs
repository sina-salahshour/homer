use std::{fs, path::PathBuf};

use anyhow::{Context, Ok, Result};
use directories::ProjectDirs;

pub trait CreateDirIfNotExists {
    fn create_if_not_exists(&self) -> Result<()>;
}

impl CreateDirIfNotExists for PathBuf {
    fn create_if_not_exists(&self) -> Result<()> {
        if !self.exists() {
            fs::create_dir_all(&self)?
        }
        Ok(())
    }
}

pub fn load_project_dirs() -> Result<ProjectDirs> {
    let dirs = ProjectDirs::from("dev", "sina-salahshour", "homer")
        .context("Couldn't process project directories")?;
    Ok(dirs)
}
