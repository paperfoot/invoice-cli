// ═══════════════════════════════════════════════════════════════════════════
// Thin shim over finance-core — preserves the invoice-cli call-site API
// (`config::load`, `config::config_path`, `config::db_path`, etc.) so the
// rest of the crate doesn't have to learn finance-core's module layout.
//
// Real storage and path resolution live in finance-core:
//   - Settings struct       → finance_core::settings::Settings (aliased here)
//   - Shared config.toml    → finance_core::paths::Paths::config_file()
//   - Shared SQLite db      → finance_core::paths::Paths::db_file()
//   - Asset root (Typst)    → finance_core::paths::Paths::assets_dir()
// ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;

use finance_core::paths::Paths;
pub use finance_core::settings::Settings as Config;

use crate::error::Result;

fn paths() -> Result<Paths> {
    Ok(Paths::resolve()?)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(paths()?.config_file())
}

pub fn state_path() -> Result<PathBuf> {
    Ok(paths()?.data_dir)
}

pub fn db_path() -> Result<PathBuf> {
    Ok(paths()?.db_file())
}

pub fn assets_path() -> Result<PathBuf> {
    Ok(paths()?.assets_dir())
}

pub fn load() -> Result<Config> {
    let p = paths()?;
    Ok(Config::load(&p)?)
}

pub fn ensure_dirs() -> Result<()> {
    let p = paths()?;
    std::fs::create_dir_all(p.assets_dir())?;
    Ok(())
}
