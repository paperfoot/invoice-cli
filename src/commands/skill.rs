use crate::cli::SkillCmd;
use crate::error::Result;
use crate::output::{print_success, Ctx};

const SKILL_MD: &str = r#"---
name: invoice-cli
description: >
  Generate beautiful, internationally-compliant invoices (PDF) from the CLI.
  Stateful (SQLite) — supports multiple issuer companies, clients, products,
  tax profiles (SG GST / UK VAT / US / EU / custom), multiple Typst templates.
  Use when the user asks to create, list, render, mark paid, or manage
  invoices, or to manage clients / products / invoicing entities.
---

## invoice-cli

`invoice` is a stateful CLI for generating, tracking, and rendering invoices.

### Quick start

```
invoice issuer add acme --name "Acme Studio" --jurisdiction sg --tax-registered --tax-id "GST M2-..." --address "..."
invoice clients add meridian --name "Meridian & Co." --country US --address "..." \
    --default-issuer acme --default-template boutique
invoice products add design --description "Design engagement" --unit project --price 8400 --currency SGD --tax-rate 9
invoice invoices new --client meridian --item design --due 30d   # no --as needed: uses client default
invoice invoices render 2026-0001 --open                          # uses client.default_template
invoice invoices mark 2026-0001 paid                              # auto-stamps paid_at
invoice invoices duplicate 2026-0001                              # clone for next month's billing
```

### Editing existing records

```
invoice issuer edit acme --phone "+65 ..." --bank-iban "SG..."
invoice clients edit meridian --default-template tiefletter-gold
invoice products edit design --price 9200
invoice issuer set-template acme boutique    # shorthand for --template
invoice clients set-issuer meridian acme     # shorthand
```

### Tips

- Run `invoice agent-info` for the full JSON capability manifest.
- Run `invoice doctor` to verify typst is installed & DB is ready.
- Item spec supports `product-slug[:qty]` OR `description:qty:price[:rate]`.
- Template resolution at render: `--template` flag > client.default_template > issuer.default_template > `vienna`.
- `--as` picks the issuer; omit it when the client has a `default_issuer` pinned.
- `mark issued` / `mark paid` auto-stamp `issued_at` / `paid_at` (first transition only).
- `invoices list` shows totals per invoice (computed with `rust_decimal`).
- Every tax value is computed with `rust_decimal` — no float rounding.
"#;

pub fn run(_cmd: SkillCmd, ctx: Ctx) -> Result<()> {
    let targets = [
        dirs_path(".claude/skills/invoice-cli/SKILL.md"),
        dirs_path(".codex/skills/invoice-cli/SKILL.md"),
        dirs_path(".gemini/skills/invoice-cli/SKILL.md"),
    ];
    let mut written = Vec::new();
    for t in targets {
        if let Some(parent) = t.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&t, SKILL_MD)?;
        written.push(t.display().to_string());
    }

    print_success(ctx, &written, |paths| {
        for p in paths {
            println!("installed → {}", p);
        }
    });
    Ok(())
}

fn dirs_path(rel: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(rel)
}
