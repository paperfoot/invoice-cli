use std::path::PathBuf;
use std::process::Command;

use crate::cli::TemplateCmd;
use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};
use crate::typst_assets;

pub fn run(cmd: TemplateCmd, ctx: Ctx) -> Result<()> {
    match cmd {
        TemplateCmd::List => {
            let names = typst_assets::list_templates()?;
            print_success(ctx, &names, |names| {
                if names.is_empty() {
                    println!("no templates found. run: invoice doctor");
                }
                for n in names {
                    println!("{n}");
                }
            });
            Ok(())
        }
        TemplateCmd::Preview { name, out } => {
            typst_assets::ensure_extracted()?;
            if !typst_assets::has_template(&name)? {
                return Err(AppError::InvalidInput(format!(
                    "template '{name}' not found. Run: invoice template list"
                )));
            }
            let template_path = typst_assets::template_path(&name)?;
            let root = typst_assets::project_root()?;
            let out_path = out
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("preview-{}.pdf", name)));

            let status = Command::new("typst")
                .arg("compile")
                .arg("--root")
                .arg(&root)
                .arg(&template_path)
                .arg(&out_path)
                .status()
                .map_err(|e| AppError::Render(format!("typst binary not found: {e}")))?;

            if !status.success() {
                return Err(AppError::Render(format!(
                    "typst compile exited with {}",
                    status.code().unwrap_or(-1)
                )));
            }

            print_success(
                ctx,
                &serde_json::json!({"template": name, "path": out_path}),
                |_| println!("rendered → {}", out_path.display()),
            );
            Ok(())
        }
    }
}
