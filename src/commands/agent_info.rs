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

    // Built outside the json! macro to avoid hitting the proc-macro recursion
    // limit as the command surface grows.
    let commands_list: &[(&str, &str)] = &[
        ("issuer add <slug> --name X --jurisdiction sg|uk|us|eu --address ... [--logo PATH]", "Register an issuer (billing entity). --logo points to a PNG/SVG/JPG rendered in template header"),
        ("issuer edit <slug> [--name ... --template ... --jurisdiction ... --logo PATH etc]", "Update any subset of an issuer's fields (incl. logo path)"),
        ("issuer set-template <slug> <template>", "Shorthand: change an issuer's default template"),
        ("issuer list | ls", "List issuers"),
        ("issuer show <slug> | get", "Show issuer details"),
        ("issuer delete <slug> | rm", "Delete an issuer"),
        ("clients add <slug> --name X --address ... [--default-issuer S --default-template T]", "Register a client, optionally pinning a default issuer/template"),
        ("clients edit <slug> [--name ... --default-issuer ... --default-template ...]", "Update any subset of a client's fields"),
        ("clients set-issuer <slug> <issuer-slug>", "Shorthand: pin the default issuer for this client"),
        ("clients set-template <slug> <template>", "Shorthand: pin the preferred template for this client"),
        ("clients list | ls", "List clients"),
        ("clients show <slug> | get", "Show client details"),
        ("clients delete <slug> | rm", "Delete a client"),
        ("products add <slug> --description X --unit Y --price N --currency SGD", "Register a reusable product/service line"),
        ("products edit <slug> [--description ... --price ... etc]", "Update any subset of a product's fields"),
        ("products list | ls", "List products"),
        ("products show <slug> | get", "Show product details"),
        ("products delete <slug> | rm", "Delete a product"),
        ("invoices new [--as <issuer>] --client <client> --item <spec>... [--discount-rate R | --discount-fixed X]", "Create a new invoice (omit --as when client has a default_issuer). Optional invoice-level discount (percent OR fixed major-units)"),
        ("invoices edit <number> [--client ... --due ... --terms ... --notes ... --currency ... --pay-link ... --reverse-charge ... --discount-rate ... --discount-fixed ...]", "Edit DRAFT invoice metadata only — issued/paid/void invoices are immutable; use credit-note instead"),
        ("invoices items <number> add <spec> [--subtitle ... --discount-rate ... --discount-fixed ...]", "Add a line item to a DRAFT invoice (spec: 'product-slug[:qty]' OR 'Description:qty:price[:rate]')"),
        ("invoices items <number> remove <position> | rm", "Remove the line at zero-indexed position from a DRAFT invoice"),
        ("invoices items <number> edit <position> [--description ... --subtitle ... --qty ... --unit ... --price ... --tax-rate ... --discount-rate ... --discount-fixed ...]", "Edit any subset of a DRAFT invoice line's fields"),
        ("invoices credit-note <number> [--full | --item <spec>...] [--notes ... --pay-link ...]", "Issue a credit note against an existing invoice. --full clones source items as positive reversal; --item lets you specify exact refund lines"),
        ("invoices aging [--as <issuer>]", "Ageing report for unpaid invoices, bucketed 0-30 / 31-60 / 61-90 / 90+ days past due"),
        ("invoices export [--from YYYY-MM-DD --to YYYY-MM-DD --format csv|json --out PATH --as <issuer>]", "Export invoices for accountant handoff. Defaults to CSV on stdout when --out omitted"),
        ("invoices duplicate <number> [--client C --as I --due 30d]", "Clone an invoice's line items into a new draft (for recurring billing)"),
        ("invoices list | ls [--status X] [--as Y] [--overdue]", "List invoices (includes total per invoice). --overdue filters to past-due unpaid invoices"),
        ("invoices show <number> | get", "Show invoice details"),
        ("invoices render <number> [--template T] [--out PATH] [--open]", "Render to PDF. Template chain: --template > client.default_template > issuer.default_template > 'vienna'"),
        ("invoices mark <number> draft|issued|paid|void", "Update invoice status (auto-stamps issued_at/paid_at)"),
        ("invoices delete <number> [--force] | rm", "Delete an invoice. --force allows deleting non-draft (breaks number-sequence integrity — prefer 'mark void' or credit-note)"),
        ("template list", "List available PDF templates"),
        ("template preview <name>", "Render a template with synthetic data"),
        ("config show | path | set <key> <value>", "View / edit config"),
        ("agent-info | info", "This manifest"),
        ("doctor", "Diagnose dependencies and config"),
        ("skill install", "Install embedded Claude/Codex/Gemini skill"),
        ("update [--check]", "Self-update — queries crates.io, upgrades via brew/cargo"),
    ];
    let mut commands = serde_json::Map::new();
    for (k, v) in commands_list {
        commands.insert((*k).to_string(), serde_json::Value::String((*v).to_string()));
    }

    let manifest = serde_json::json!({
        "name": "invoice",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "commands": commands,
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
