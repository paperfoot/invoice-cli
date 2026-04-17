use std::path::PathBuf;

use directories::ProjectDirs;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_issuer: Option<String>,
    pub default_template: String,
    pub open_pdf: bool,
    pub self_update: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_issuer: None,
            default_template: "vienna".into(),
            open_pdf: true,
            self_update: true,
        }
    }
}

fn dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("com", "199-biotechnologies", "invoice")
        .ok_or_else(|| AppError::Config("could not resolve platform dirs".into()))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(dirs()?.config_dir().join("config.toml"))
}

pub fn state_path() -> Result<PathBuf> {
    Ok(dirs()?.data_local_dir().to_path_buf())
}

pub fn db_path() -> Result<PathBuf> {
    Ok(state_path()?.join("invoice.db"))
}

pub fn assets_path() -> Result<PathBuf> {
    Ok(state_path()?.join("typst"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    let config = Figment::from(figment::providers::Serialized::defaults(Config::default()))
        .merge(Toml::file(&path))
        .merge(Env::prefixed("INVOICE_"))
        .extract::<Config>()
        .map_err(|e| AppError::Config(format!("{e}")))?;
    Ok(config)
}

pub fn ensure_dirs() -> Result<()> {
    let cfg = config_path()?.parent().map(|p| p.to_path_buf());
    if let Some(p) = cfg {
        std::fs::create_dir_all(&p)?;
    }
    std::fs::create_dir_all(state_path()?)?;
    std::fs::create_dir_all(assets_path()?)?;
    Ok(())
}
