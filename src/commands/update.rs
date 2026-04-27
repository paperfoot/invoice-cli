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
        print_success(ctx, &payload, |_| {
            println!("already on latest ({})", current)
        });
        return Ok(());
    }

    // Detect install method and use the right upgrade command.
    let cmd = install_upgrade_command()?;
    eprintln!("upgrading {current} → {latest} via: {}", cmd.display());
    let mut child = Command::new(&cmd.program);
    child.args(&cmd.args);
    for (key, value) in &cmd.env {
        child.env(key, value);
    }
    let status = child
        .status()
        .map_err(|e| AppError::Other(format!("failed to launch upgrader: {e}")))?;

    if !status.success() {
        return Err(AppError::Other(format!(
            "upgrade command exited with status {}",
            status.code().unwrap_or(-1)
        )));
    }

    let installed = installed_invoice_version()?;
    if version_newer_than(&latest, &installed) {
        return Err(AppError::Other(format!(
            "upgrade completed but `invoice --version` reports {installed}, expected {latest}"
        )));
    }

    let payload = serde_json::json!({
        "current": current,
        "latest": latest,
        "installed": installed,
        "updated": true,
        "method": cmd.display(),
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

struct UpgradeCommand {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl UpgradeCommand {
    fn display(&self) -> String {
        let mut parts = Vec::with_capacity(1 + self.args.len());
        parts.push(self.program.as_str());
        parts.extend(self.args.iter().map(String::as_str));
        parts.join(" ")
    }
}

/// Pick the right upgrader. Prefer Homebrew on macOS when `brew` is on PATH
/// and the binary lives under a brew prefix; otherwise fall back to cargo.
fn install_upgrade_command() -> Result<UpgradeCommand> {
    if cfg!(target_os = "macos") && running_under_brew() {
        refresh_homebrew_tap()?;
        return Ok(UpgradeCommand {
            program: "brew".into(),
            args: vec!["upgrade".into(), "199-biotechnologies/tap/invoice".into()],
            // The tap was refreshed directly above. Avoid failing because an
            // unrelated local tap is broken during Homebrew auto-update.
            env: vec![("HOMEBREW_NO_AUTO_UPDATE".into(), "1".into())],
        });
    }
    Ok(UpgradeCommand {
        program: "cargo".into(),
        args: vec![
            "install".into(),
            "--force".into(),
            "--locked".into(),
            "invoice-cli".into(),
        ],
        env: Vec::new(),
    })
}

fn refresh_homebrew_tap() -> Result<()> {
    let repo = Command::new("brew")
        .args(["--repo", "199-biotechnologies/tap"])
        .output()
        .map_err(|e| AppError::Other(format!("failed to locate Homebrew tap: {e}")))?;
    if !repo.status.success() {
        return Err(AppError::Other(format!(
            "failed to locate Homebrew tap (exit {})",
            repo.status.code().unwrap_or(-1)
        )));
    }
    let path = String::from_utf8_lossy(&repo.stdout).trim().to_string();
    if path.is_empty() {
        return Err(AppError::Other(
            "brew --repo returned an empty tap path".into(),
        ));
    }

    eprintln!("refreshing Homebrew tap: git -C {path} pull --ff-only");
    let status = Command::new("git")
        .args(["-C", &path, "pull", "--ff-only"])
        .status()
        .map_err(|e| AppError::Other(format!("failed to refresh Homebrew tap: {e}")))?;
    if !status.success() {
        return Err(AppError::Other(format!(
            "failed to refresh Homebrew tap (exit {})",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

fn installed_invoice_version() -> Result<String> {
    let out = Command::new("invoice")
        .arg("--version")
        .output()
        .map_err(|e| AppError::Other(format!("failed to verify installed invoice: {e}")))?;
    if !out.status.success() {
        return Err(AppError::Other(format!(
            "invoice --version failed after upgrade (exit {})",
            out.status.code().unwrap_or(-1)
        )));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_invoice_version(&stdout)
        .ok_or_else(|| AppError::Other(format!("could not parse invoice version from: {stdout}")))
}

fn parse_invoice_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|part| parse_version(part).is_some())
        .map(ToOwned::to_owned)
}

fn running_under_brew() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let s = exe.to_string_lossy();
    s.contains("/homebrew/") || s.contains("/Cellar/") || s.contains("/opt/homebrew/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_invoice_version_output() {
        assert_eq!(
            parse_invoice_version("invoice 0.5.9\n").as_deref(),
            Some("0.5.9")
        );
    }

    #[test]
    fn compares_semver_versions() {
        assert!(version_newer_than("0.5.10", "0.5.9"));
        assert!(!version_newer_than("0.5.9", "0.5.9"));
        assert!(!version_newer_than("0.5.9", "0.5.10"));
    }
}
