# invoice-cli

> Beautiful invoices from the CLI — international, stateful, agent-friendly.

A stateful, single-binary CLI for generating, tracking, and rendering invoices.
Built for humans who want a clean terminal workflow *and* for AI agents that
need a deterministic JSON interface to bill clients on their owner's behalf.

## Why

Most invoice tools fall into two camps:

- **Full accounting suites** (Xero, Wave, QuickBooks) — powerful but overkill if
  you just need to send a beautifully-branded invoice.
- **Static generators** (typst templates, LaTeX invoices) — gorgeous output but
  no state: you re-type client details every time, numbering drifts, and an
  agent can't "render last month's invoice".

`invoice-cli` sits between them: a stateful SQLite store of issuers, clients,
products and invoices, a small set of beautiful Typst templates, and a JSON
interface built for agents.

## Features

- **Multi-issuer first-class.** Run several companies (SG Pte. Ltd., UK Ltd.,
  US LLC, …) from one binary. Each issuer has its own jurisdiction, tax
  profile, default template, numbering series, and logo.
- **Per-client defaults.** Pin a default issuer and/or template per client —
  then just `invoice invoices new --client meridian --item design` and the
  right entity + branding lights up automatically.
- **International tax profiles.** Built-in: Singapore GST 9%, UK VAT 20%,
  US (state-variable), EU VAT, plus a `custom` profile. Reverse-charge flag
  for EU cross-border B2B — rendered on the PDF with the legally-required
  callout.
- **Precise money math.** Amounts stored as `i64` minor units; tax math uses
  `rust_decimal` — no float rounding artefacts. Discount math clamped at
  zero so a mis-sized fixed discount can't flip totals negative.
- **Discounts** at line or invoice level (rate or fixed amount).
- **Five polished Typst templates** out of the box: `vienna`, `helvetica-nera`,
  `tiefletter-gold`, `monoline`, `boutique`. Self-contained — single binary,
  templates are embedded and extracted on first use.
- **Logos per issuer.** Attach a PNG/SVG/JPG and each template renders it in
  the header at the appropriate size for its design language.
- **Credit notes.** Issue against any existing invoice with `credit-note
  --full` (full reversal) or `--item ...` (specific refund lines). Independent
  `CN-YYYY-NNNN` numbering series so credit notes don't collide with invoices.
- **Draft-only editing.** Amend a draft's metadata with `invoices edit` or
  its line items with `invoices items add|remove|edit`. Once issued, invoices
  are immutable — the correct path for corrections is a credit note, which
  preserves the audit trail and number-sequence integrity required by
  SG/EU/UK regulations.
- **QR pay-links.** Set `--pay-link https://buy.stripe.com/...` on an invoice
  and the renderer stamps a scan-to-pay QR on the PDF.
- **Lifecycle timestamps.** `mark issued` / `mark paid` auto-stamp
  `issued_at` / `paid_at` (idempotent, first-transition-only).
- **Aging + CSV export.** `invoices aging` buckets unpaid invoices into
  0-30/31-60/61-90/90+ days overdue. `invoices export --from X --to Y
  --format csv` gives you a clean accountant handoff.
- **Agent-friendly.** Every command emits a JSON envelope when piped or
  `--json`; `invoice agent-info` returns a capability manifest; structured
  error codes with suggestions; exit codes distinguish transient vs permanent
  failures. Install the embedded Claude/Codex/Gemini skill with
  `invoice skill install`.

## Install

### Homebrew (macOS / Linux)

```
brew tap 199-biotechnologies/tap
brew install invoice
```

### Cargo

```
cargo install invoice-cli
```

### From source

```
git clone https://github.com/199-biotechnologies/invoice-cli
cd invoice-cli
cargo install --path .
```

All install paths produce a single `invoice` binary. Typst is the only runtime
dependency (`brew install typst` on macOS).

## Quick start

```
# 1. Register your billing entity (with logo + bank details)
invoice issuer add acme \
    --name "Acme Studio" --jurisdiction sg --tax-registered \
    --tax-id "M2-1234567-8" --address "1 Marina Bay\nSingapore 018989" \
    --template boutique --logo ~/Pictures/acme.png \
    --bank-name "DBS" --bank-iban "SG11DBSS..." --bank-bic "DBSSSGSG"

# 2. Add a client, pinning acme as their default issuer
invoice clients add meridian \
    --name "Meridian & Co." --country US \
    --address "530 5th Ave\nNew York, NY 10036" \
    --default-issuer acme --default-template boutique

# 3. Register a reusable line item
invoice products add design \
    --description "Creative direction" --unit project \
    --price 8400 --currency SGD --tax-rate 9

# 4. Create an invoice — no --as needed, uses client's default issuer
invoice invoices new --client meridian --item design --due 30d

# 5. Render + open
invoice invoices render 2026-0001 --open

# 6. Later: mark paid, clone for next month
invoice invoices mark 2026-0001 paid
invoice invoices duplicate 2026-0001

# 7. Need a refund? Credit note against the original:
invoice invoices credit-note 2026-0001 --item "Refund:1:500" --notes "Goodwill credit"

# 8. Month-end accountant handoff
invoice invoices export --from 2026-01-01 --to 2026-03-31 --format csv --out q1.csv
```

## Core commands

| Command | Purpose |
|---|---|
| `issuer add\|edit\|list\|show\|delete` | Manage billing entities (your companies) |
| `issuer set-template <slug> <tmpl>` | Shorthand to change an issuer's default template |
| `clients add\|edit\|list\|show\|delete` | Manage clients (who you bill) |
| `clients set-issuer <slug> <issuer>` | Pin default issuer for a client |
| `clients set-template <slug> <tmpl>` | Pin default template for a client |
| `products add\|edit\|list\|show\|delete` | Manage reusable line items |
| `invoices new --client X --item Y` | Create a new invoice |
| `invoices edit <number>` | Edit draft metadata (due, terms, notes, discount…) |
| `invoices items add\|remove\|edit <number>` | Mutate line items on a draft invoice |
| `invoices duplicate <number>` | Clone an invoice as a fresh draft (recurring billing) |
| `invoices credit-note <number>` | Issue a credit note against an existing invoice |
| `invoices render <number> [--template T] [--open]` | Generate PDF |
| `invoices mark <number> draft\|issued\|paid\|void` | Update status (auto-stamps timestamps) |
| `invoices list [--status X] [--as Y] [--overdue]` | List invoices with totals |
| `invoices aging [--as Y]` | Aging buckets for unpaid invoices |
| `invoices export --from X --to Y --format csv\|json` | Accountant handoff |
| `invoices delete <number> [--force]` | Delete an invoice (`--force` for non-draft) |
| `template list\|preview <name>` | Inspect available templates |
| `doctor` | Diagnose typst install, DB, templates |
| `agent-info` | Full JSON capability manifest |
| `skill install` | Install embedded Claude/Codex/Gemini skill |
| `update [--check]` | Self-update via brew or cargo |

Run `invoice --help` for the full reference or `invoice <subcommand> --help`
for any subcommand.

## Template resolution

At render time, the template chain is:

```
--template flag  >  client.default_template  >  issuer.default_template  >  "vienna"
```

So pinning a template on a client gives them consistent branding without you
having to pass `--template` every time.

## Item specs

On `invoices new`, each `--item` is one of:

- `product-slug` — uses the product's price, unit, and tax rate
- `product-slug:qty` — e.g. `design:2` for two units
- `"Description:qty:price"` — ad-hoc item with default tax rate from jurisdiction
- `"Description:qty:price:rate"` — ad-hoc item with explicit tax rate

## State & privacy

- **Config:** `~/.config/invoice/config.toml` (Linux) or
  `~/Library/Application Support/com.199-biotechnologies.invoice/` (macOS)
- **Database:** `~/.local/share/invoice/invoice.db` (SQLite, WAL mode)
- **Templates:** extracted once to the state dir, refreshed on upgrade

Nothing ever leaves your machine unless you choose to — no telemetry, no
phone-home, no cloud sync.

## Agent usage

This CLI is designed to be driven by AI agents as well as humans. The
contract:

- Every command emits a `{version, status, data|error}` envelope when piped.
- `invoice agent-info` returns a full capability + exit-code manifest.
- `invoice skill install` drops a ready-to-use skill file into
  `~/.claude/skills/invoice-cli/SKILL.md` (and the Codex/Gemini equivalents).

Typical agent workflow:

```
USER:  "Bill Meridian for last month's design work"
AGENT: invoice invoices duplicate $(invoice invoices list --json | jq -r '.data[0].number')
```

## Architecture

- **Rust** binary via `cargo` / single-binary distribution.
- **SQLite** via `rusqlite` with `refinery` migrations (`migrations/V*.sql`).
- **Typst** for PDF rendering — templates live in `typst/`, embedded via
  `rust-embed` and extracted to the user's state dir.
- **Money** as `i64` minor units; tax with `rust_decimal`. No floats in
  financial paths.
- **Built on** [`agent-cli-framework`](https://github.com/199-biotechnologies/agent-cli-framework)
  (ACF) conventions for agent ergonomics.

## Scope

This is an **invoicing tool**, not an accounting suite. In scope:

- Clean, branded invoice generation across multiple entities
- Multi-currency, multi-jurisdiction tax handling
- Lifecycle tracking (draft → issued → paid → void)
- Recurring billing via `duplicate`

Explicitly out of scope:

- Double-entry bookkeeping
- Bills payable / accounts payable
- Bank reconciliation
- GST/VAT filing reports
- Chart of accounts, P&L, balance sheet
- Payroll, inventory, expense tracking

For those, reach for Xero, Wave, or QuickBooks.

## License

MIT © 199 Biotechnologies
