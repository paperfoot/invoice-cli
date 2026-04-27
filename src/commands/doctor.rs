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

    // db + suite-level invariants
    match crate::db::open() {
        Ok(conn) => {
            checks.push(Check {
                name: "database".into(),
                status: "pass",
                message: format!("{} ok", config::db_path()?.display()),
            });
            add_suite_checks(&mut checks, &conn)?;
        }
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

fn add_suite_checks(checks: &mut Vec<Check>, conn: &rusqlite::Connection) -> Result<()> {
    let issuers = crate::db::issuer_list(conn)?;
    if issuers.is_empty() {
        checks.push(Check {
            name: "issuers".into(),
            status: "warn",
            message: "no issuers configured. First run: invoice issuer add <slug> --name ... --address ...".into(),
        });
    } else {
        checks.push(Check {
            name: "issuers".into(),
            status: "pass",
            message: format!("{} configured", issuers.len()),
        });
    }

    let cfg = config::load()?;
    match cfg.default_issuer.as_deref() {
        Some(slug) => match crate::db::issuer_by_slug(conn, slug) {
            Ok(_) => checks.push(Check {
                name: "default-issuer".into(),
                status: "pass",
                message: format!("config.default_issuer = {slug}"),
            }),
            Err(_) => checks.push(Check {
                name: "default-issuer".into(),
                status: "fail",
                message: format!(
                    "config.default_issuer points to missing issuer '{slug}'. Run: invoice config set default_issuer unset"
                ),
            }),
        },
        None if issuers.is_empty() => checks.push(Check {
            name: "default-issuer".into(),
            status: "warn",
            message: "not set yet because no issuers exist".into(),
        }),
        None => checks.push(Check {
            name: "default-issuer".into(),
            status: "warn",
            message: "not set. Agents must pass --as or use clients with default issuers.".into(),
        }),
    }

    if issuers.len() > 1 {
        use std::collections::BTreeMap;
        let mut by_format: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for issuer in &issuers {
            by_format
                .entry(issuer.number_format.as_str())
                .or_default()
                .push(issuer.slug.as_str());
        }
        let risky: Vec<String> = by_format
            .into_iter()
            .filter(|(format, slugs)| slugs.len() > 1 && !format.contains("{issuer}"))
            .map(|(format, slugs)| format!("{} share '{}'", slugs.join(", "), format))
            .collect();
        if risky.is_empty() {
            checks.push(Check {
                name: "numbering".into(),
                status: "pass",
                message: "multi-company number formats are distinct or include {issuer}".into(),
            });
        } else {
            checks.push(Check {
                name: "numbering".into(),
                status: "warn",
                message: format!(
                    "{}. Invoice numbers are globally addressable; use --number-format '{{issuer}}-{{year}}-{{seq:04}}' for each issuer.",
                    risky.join("; ")
                ),
            });
        }
    }

    Ok(())
}
