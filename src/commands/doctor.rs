use std::process::Command;

use crate::config;
use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};
use crate::typst_assets;

#[derive(serde::Serialize)]
pub struct DoctorReport {
    checks: Vec<Check>,
    summary: Summary,
}

#[derive(serde::Serialize)]
pub struct Check {
    name: String,
    status: &'static str, // "pass" | "warn" | "fail"
    message: String,
}

#[derive(serde::Serialize)]
pub struct Summary {
    pass: usize,
    warn: usize,
    fail: usize,
}

pub fn run(ctx: Ctx) -> Result<()> {
    let mut checks = Vec::new();

    // typst binary
    match Command::new("typst").arg("--version").output() {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            checks.push(Check {
                name: "typst".into(),
                status: "pass",
                message: v,
            });
        }
        _ => checks.push(Check {
            name: "typst".into(),
            status: "fail",
            message: "typst binary not found on PATH. Install: brew install typst".into(),
        }),
    }

    // config dir
    let cfg_path = config::config_path()?;
    checks.push(Check {
        name: "config".into(),
        status: "pass",
        message: format!("{} (exists: {})", cfg_path.display(), cfg_path.exists()),
    });

    // state dir
    let state = config::state_path()?;
    checks.push(Check {
        name: "state-dir".into(),
        status: "pass",
        message: format!("{} (exists: {})", state.display(), state.exists()),
    });

    // templates extracted
    typst_assets::ensure_extracted()?;
    let templates = typst_assets::list_templates()?;
    checks.push(Check {
        name: "templates".into(),
        status: if templates.is_empty() { "fail" } else { "pass" },
        message: format!("{} available: {}", templates.len(), templates.join(", ")),
    });

    // db
    match crate::db::open() {
        Ok(_) => checks.push(Check {
            name: "database".into(),
            status: "pass",
            message: format!("{} ok", config::db_path()?.display()),
        }),
        Err(e) => checks.push(Check {
            name: "database".into(),
            status: "fail",
            message: format!("{e}"),
        }),
    }

    let summary = Summary {
        pass: checks.iter().filter(|c| c.status == "pass").count(),
        warn: checks.iter().filter(|c| c.status == "warn").count(),
        fail: checks.iter().filter(|c| c.status == "fail").count(),
    };
    let has_fail = summary.fail > 0;
    let report = DoctorReport { checks, summary };

    print_success(ctx, &report, |r| {
        for c in &r.checks {
            let icon = match c.status {
                "pass" => "✓",
                "warn" => "!",
                "fail" => "✗",
                _ => "?",
            };
            eprintln!("  {} {:<16}  {}", icon, c.name, c.message);
        }
        eprintln!(
            "\n{} passing, {} warnings, {} failing",
            r.summary.pass, r.summary.warn, r.summary.fail
        );
    });

    if has_fail {
        return Err(AppError::Config("doctor found issues".into()));
    }
    Ok(())
}
