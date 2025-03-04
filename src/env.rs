use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

use crate::fonts::{FontInfo, FontRepo, NerdFontsInfo};

#[derive(Serialize, Deserialize)]
pub struct Env {
    pub font: EnvFont,
}

#[derive(Serialize, Deserialize)]
pub struct EnvFont {
    repo_url: String,
    root_url: String,
    #[serde(default = "default_fonts_dir_name")]
    pub fonts_dir_name: String,
}

fn default_fonts_dir_name() -> String {
    "$homer".into()
}

impl Env {
    pub fn prepare() -> Result<Self> {
        let env_raw = include_str!("../assets/constants.json");
        let env: Env = serde_json::from_str(&env_raw)?;
        Ok(env)
    }
}

impl EnvFont {
    pub async fn fetch_fonts(&self) -> Result<Vec<FontInfo>> {
        let res = FontRepo::fetch(&self.repo_url).await?.fonts;
        Ok(res)
    }
    pub async fn fetch_root(&self) -> Result<NerdFontsInfo> {
        NerdFontsInfo::fetch(&self.root_url).await
    }
}
