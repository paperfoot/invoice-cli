use crate::error::Result;
use crate::output::{print_raw, Ctx};
use crate::tax;

pub fn run(_ctx: Ctx) -> Result<()> {
    let profiles: Vec<_> = tax::all_profiles()
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "code": p.code,
                "country": p.country,
                "tax_label": p.tax_label,
                "default_rate": p.default_rate,
                "currency": p.currency,
                "symbol": p.symbol,
                "tax_invoice_title": p.tax_invoice_title,
                "supports_reverse_charge": p.supports_reverse_charge,
            })
        })
        .collect();

    let manifest = serde_json::json!({
        "name": "invoice",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "commands": {
            "issuer add <slug> --name X --jurisdiction sg|uk|us|eu --address ...": "Register an issuer (billing entity)",
            "issuer edit <slug> [--name ... --template ... --jurisdiction ... etc]": "Update any subset of an issuer's fields",
            "issuer set-template <slug> <template>": "Shorthand: change an issuer's default template",
            "issuer list | ls": "List issuers",
            "issuer show <slug> | get": "Show issuer details",
            "issuer delete <slug> | rm": "Delete an issuer",
            "clients add <slug> --name X --address ... [--default-issuer S --default-template T]": "Register a client, optionally pinning a default issuer/template",
            "clients edit <slug> [--name ... --default-issuer ... --default-template ...]": "Update any subset of a client's fields",
            "clients set-issuer <slug> <issuer-slug>": "Shorthand: pin the default issuer for this client",
            "clients set-template <slug> <template>": "Shorthand: pin the preferred template for this client",
            "clients list | ls": "List clients",
            "clients show <slug> | get": "Show client details",
            "clients delete <slug> | rm": "Delete a client",
            "products add <slug> --description X --unit Y --price N --currency SGD": "Register a reusable product/service line",
            "products edit <slug> [--description ... --price ... etc]": "Update any subset of a product's fields",
            "products list | ls": "List products",
            "products show <slug> | get": "Show product details",
            "products delete <slug> | rm": "Delete a product",
            "invoices new [--as <issuer>] --client <client> --item <spec>...": "Create a new invoice (omit --as when client has a default_issuer)",
            "invoices duplicate <number> [--client C --as I --due 30d]": "Clone an invoice's line items into a new draft (for recurring billing)",
            "invoices list | ls [--status X] [--as Y]": "List invoices (includes total per invoice)",
            "invoices show <number> | get": "Show invoice details",
            "invoices render <number> [--template T] [--out PATH] [--open]": "Render to PDF. Template chain: --template > client.default_template > issuer.default_template > 'vienna'",
            "invoices mark <number> draft|issued|paid|void": "Update invoice status (auto-stamps issued_at/paid_at)",
            "invoices delete <number> | rm": "Delete an invoice",
            "template list": "List available PDF templates",
            "template preview <name>": "Render a template with synthetic data",
            "config show | path | set <key> <value>": "View / edit config",
            "agent-info | info": "This manifest",
            "doctor": "Diagnose dependencies and config",
            "skill install": "Install embedded Claude/Codex/Gemini skill",
            "update [--check]": "Self-update — queries crates.io, upgrades via brew/cargo"
        },
        "flags": {
            "--json": "Force JSON envelope output (auto-enabled when piped)",
            "--quiet": "Suppress human output"
        },
        "exit_codes": {
            "0": "Success",
            "1": "Transient error (IO, render) — retry may help",
            "2": "Config error — fix setup",
            "3": "Bad input / not found / ambiguous — fix arguments",
            "4": "Rate limited — wait and retry"
        },
        "envelope_schema": {
            "version": "1",
            "status": "success | error",
            "data": "… (success)",
            "error": "{ code, message, suggestion } (error)"
        },
        "config_path": "~/.config/invoice/config.toml",
        "state_dir": "~/.local/share/invoice/",
        "database": "~/.local/share/invoice/invoice.db",
        "templates": ["helvetica-nera", "tiefletter-gold", "monoline", "vienna", "boutique"],
        "tax_profiles": profiles,
        "item_spec": "product-slug[:qty]  OR  description:qty:price[:rate]"
    });

    print_raw(&manifest);
    Ok(())
}
