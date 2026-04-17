use std::process::Command;

use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};

const CRATES_IO_URL: &str = "https://crates.io/api/v1/crates/invoice-cli";

pub fn run(ctx: Ctx, check: bool) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = match fetch_latest_version() {
        Ok(v) => v,
        Err(e) => {
            // Crate not yet published, or crates.io unreachable. Not fatal for
            // --check — report current and move on.
            let payload = serde_json::json!({
                "current": current,
                "latest": null,
                "update_available": false,
                "note": format!("could not query crates.io: {e}. You may not be on a published release yet."),
            });
            print_success(ctx, &payload, |p| {
                println!("current: {}", p["current"].as_str().unwrap_or("?"));
                println!("latest:  unknown ({})", p["note"].as_str().unwrap_or(""));
            });
            return Ok(());
        }
    };

    let is_newer = version_newer_than(&latest, current);

    if check {
        let payload = serde_json::json!({
            "current": current,
            "latest": latest,
            "update_available": is_newer,
        });
        print_success(ctx, &payload, |p| {
            println!("current: {}", p["current"].as_str().unwrap_or("?"));
            println!("latest:  {}", p["latest"].as_str().unwrap_or("?"));
            if is_newer {
                println!("update available — run `invoice update` to install");
            } else {
                println!("up to date");
            }
        });
        return Ok(());
    }

    if !is_newer {
        let payload = serde_json::json!({
            "current": current,
            "latest": latest,
            "updated": false,
            "note": "already on latest",
        });
        print_success(ctx, &payload, |_| println!("already on latest ({})", current));
        return Ok(());
    }

    // Detect install method and use the right upgrade command.
    let cmd = install_upgrade_command();
    eprintln!("upgrading {current} → {latest} via: {}", cmd.join(" "));
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .map_err(|e| AppError::Other(format!("failed to launch upgrader: {e}")))?;

    if !status.success() {
        return Err(AppError::Other(format!(
            "upgrade command exited with status {}",
            status.code().unwrap_or(-1)
        )));
    }

    let payload = serde_json::json!({
        "current": current,
        "latest": latest,
        "updated": true,
        "method": cmd.join(" "),
    });
    print_success(ctx, &payload, |_| {
        println!("upgraded to {latest}. verify with: invoice --version")
    });
    Ok(())
}

fn fetch_latest_version() -> Result<String> {
    let out = Command::new("curl")
        .args([
            "-sSL",
            "-H",
            "User-Agent: invoice-cli",
            "-H",
            "Accept: application/json",
            CRATES_IO_URL,
        ])
        .output()
        .map_err(|e| AppError::Other(format!("curl not available: {e}")))?;
    if !out.status.success() {
        return Err(AppError::Other(format!(
            "crates.io query failed (exit {})",
            out.status.code().unwrap_or(-1)
        )));
    }
    let body: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| AppError::Other(format!("bad crates.io response: {e}")))?;
    // crates.io 404 shape: { "errors": [{ "detail": "…" }] }
    if let Some(errors) = body.get("errors").and_then(|e| e.as_array()) {
        let detail = errors
            .first()
            .and_then(|e| e.get("detail"))
            .and_then(|d| d.as_str())
            .unwrap_or("unknown");
        return Err(AppError::Other(format!("crates.io: {detail}")));
    }
    body.get("crate")
        .and_then(|c| c.get("max_stable_version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Other("crates.io response missing max_stable_version".into()))
}

/// Semver-aware comparison: is `a` strictly newer than `b`? Falls back to
/// string compare on parse failure (pessimistic: returns false).
fn version_newer_than(a: &str, b: &str) -> bool {
    let pa = parse_version(a);
    let pb = parse_version(b);
    match (pa, pb) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next().unwrap_or("0").parse().unwrap_or(0),
    ))
}

/// Pick the right upgrader. Prefer Homebrew on macOS when `brew` is on PATH
/// and the binary lives under a brew prefix; otherwise fall back to cargo.
fn install_upgrade_command() -> Vec<String> {
    if cfg!(target_os = "macos") && running_under_brew() {
        return vec![
            "brew".into(),
            "upgrade".into(),
            "199-biotechnologies/tap/invoice".into(),
        ];
    }
    vec![
        "cargo".into(),
        "install".into(),
        "--force".into(),
        "invoice-cli".into(),
    ]
}

fn running_under_brew() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let s = exe.to_string_lossy();
    s.contains("/homebrew/") || s.contains("/Cellar/") || s.contains("/opt/homebrew/")
}
