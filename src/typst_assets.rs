// ═══════════════════════════════════════════════════════════════════════════
// Embedded Typst sources. Extracted into the shared accounting-suite assets
// directory on first use so templates can reference each other via relative
// #import paths.
// ═══════════════════════════════════════════════════════════════════════════

use rust_embed::RustEmbed;
use std::path::Path;

use crate::config;
use crate::error::Result;

#[derive(RustEmbed)]
#[folder = "typst/"]
#[prefix = ""]
pub struct Assets;

pub fn ensure_extracted() -> Result<()> {
    let root = config::assets_path()?;
    std::fs::create_dir_all(&root)?;
    for path in Assets::iter() {
        let file = Assets::get(&path).expect("embedded asset");
        let dest = root.join(path.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Only overwrite if content differs or file is missing
        let needs_write = match std::fs::read(&dest) {
            Ok(existing) => existing != file.data.as_ref(),
            Err(_) => true,
        };
        if needs_write {
            std::fs::write(&dest, file.data.as_ref())?;
        }
    }
    Ok(())
}

pub fn template_dir() -> Result<std::path::PathBuf> {
    Ok(config::assets_path()?.join("templates"))
}

pub fn shared_dir() -> Result<std::path::PathBuf> {
    Ok(config::assets_path()?.join("shared"))
}

pub fn template_path(name: &str) -> Result<std::path::PathBuf> {
    Ok(template_dir()?.join(format!("{name}.typ")))
}

pub fn list_templates() -> Result<Vec<String>> {
    let dir = template_dir()?;
    let mut names = Vec::new();
    if dir.exists() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("typ") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip stress variants
                    if !name.ends_with("-stress") {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn has_template(name: &str) -> Result<bool> {
    Ok(template_path(name)?.exists())
}

pub fn project_root() -> Result<std::path::PathBuf> {
    config::assets_path()
}

pub fn is_within_root(path: &Path) -> Result<bool> {
    let root = project_root()?;
    Ok(path.starts_with(&root))
}
