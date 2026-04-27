use crate::cli::ConfigCmd;
use crate::config;
use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};

pub fn run(cmd: ConfigCmd, ctx: Ctx) -> Result<()> {
    match cmd {
        ConfigCmd::Show => {
            let cfg = config::load()?;
            print_success(ctx, &cfg, |c| println!("{:#?}", c));
            Ok(())
        }
        ConfigCmd::Path => {
            let p = config::config_path()?;
            print_success(ctx, &p, |p| println!("{}", p.display()));
            Ok(())
        }
        ConfigCmd::Set { key, value } => {
            let remove_key = matches!(
                value.to_ascii_lowercase().as_str(),
                "unset" | "none" | "null" | ""
            );
            validate_config_value(&key, &value, remove_key)?;

            // Minimal implementation: set-and-save via toml::Value merge.
            let path = config::config_path()?;
            let existing = if path.exists() {
                std::fs::read_to_string(&path)?
            } else {
                String::new()
            };
            let mut doc: toml::Value =
                toml::from_str(&existing).unwrap_or(toml::Value::Table(Default::default()));
            if let toml::Value::Table(ref mut t) = doc {
                if remove_key {
                    t.remove(&key);
                } else {
                    t.insert(key.clone(), parse_value(&value));
                }
            } else {
                return Err(AppError::Config("config root is not a table".into()));
            }
            std::fs::write(&path, toml::to_string_pretty(&doc).unwrap())?;
            print_success(
                ctx,
                &serde_json::json!({"key": key, "value": if remove_key { serde_json::Value::Null } else { serde_json::Value::String(value.clone()) }}),
                |_| {
                    if remove_key {
                        println!("unset {key}");
                    } else {
                        println!("set {key} = {value}");
                    }
                },
            );
            Ok(())
        }
    }
}

fn validate_config_value(key: &str, value: &str, remove_key: bool) -> Result<()> {
    if key == "default_issuer" && !remove_key {
        let conn = crate::db::open()?;
        crate::db::issuer_by_slug(&conn, value).map_err(|_| {
            AppError::InvalidInput(format!(
                "unknown issuer '{value}' for config.default_issuer. Add it first or run `invoice issuer list`."
            ))
        })?;
    }
    Ok(())
}

fn parse_value(v: &str) -> toml::Value {
    if v == "true" {
        toml::Value::Boolean(true)
    } else if v == "false" {
        toml::Value::Boolean(false)
    } else if let Ok(n) = v.parse::<i64>() {
        toml::Value::Integer(n)
    } else {
        toml::Value::String(v.to_string())
    }
}
