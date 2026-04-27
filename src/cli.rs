use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "invoice", version, about = "Beautiful invoices from the CLI")]
pub struct Cli {
    /// Emit JSON envelope on stdout (auto-detected when piped)
    #[arg(long, global = true)]
    pub json: bool,
    /// Suppress human output
    #[arg(long, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage issuers (the companies you invoice AS — supports multiple)
    #[command(visible_alias = "issuer", subcommand)]
    Issuers(IssuerCmd),

    /// Manage clients (the companies you invoice TO)
    #[command(subcommand)]
    Clients(ClientCmd),

    /// Manage reusable products/line-items
    #[command(subcommand)]
    Products(ProductCmd),

    /// Create, list, show, render, or mark invoices
    #[command(subcommand)]
    Invoices(InvoiceCmd),

    /// Template operations (list, preview, set default)
    #[command(subcommand)]
    Template(TemplateCmd),

    /// Show / edit config
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Self-describing JSON manifest for agents
    #[command(alias = "info")]
    AgentInfo,

    /// Install the embedded skill file to ~/.claude/skills/
    #[command(subcommand)]
    Skill(SkillCmd),

    /// Run dependency & config diagnostics
    Doctor,

    /// Self-update from GitHub Releases
    Update {
        /// Don't install, just report latest version
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum IssuerCmd {
    /// Add a new issuer
    #[command(alias = "new")]
    Add {
        slug: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        legal_name: Option<String>,
        #[arg(long, default_value = "sg")]
        jurisdiction: String,
        #[arg(long)]
        tax_registered: bool,
        #[arg(long)]
        tax_id: Option<String>,
        #[arg(long)]
        company_no: Option<String>,
        #[arg(long)]
        address: String,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        phone: Option<String>,
        /// Bank / payment detail line as "Label: Value". Repeat for each
        /// line. Example:
        ///   --bank-line "Bank: DBS" --bank-line "Account: 1234567890"
        ///   --bank-line "Bank Code: 7171" --bank-line "SWIFT: DBSSSGSG"
        /// Lines render as a two-column list on the invoice PDF.
        #[arg(long = "bank-line")]
        bank_line: Vec<String>,
        #[arg(long, default_value = "vienna")]
        template: String,
        /// Path to a logo image (PNG/SVG/JPG). Rendered in template header.
        #[arg(long)]
        logo: Option<String>,
        /// Default directory for `invoices render` output when --out is
        /// omitted. Leading `~/` is expanded. Example:
        ///   --output-dir "~/Documents/Invoices/Paperfoot"
        #[arg(long)]
        output_dir: Option<String>,
        /// Default notes auto-populated into new invoices (free-form
        /// multi-line). Use for payment terms, reverse-charge disclaimers,
        /// etc.
        #[arg(long)]
        notes: Option<String>,
        /// Invoice number format. Tokens: {issuer}, {year}, {seq}, {seq:04}.
        /// Default includes {issuer} so multiple companies cannot collide.
        #[arg(long)]
        number_format: Option<String>,
    },
    /// Edit an existing issuer — pass only the fields you want to change
    Edit {
        slug: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        legal_name: Option<String>,
        #[arg(long)]
        jurisdiction: Option<String>,
        #[arg(long)]
        tax_registered: Option<bool>,
        #[arg(long)]
        tax_id: Option<String>,
        #[arg(long)]
        company_no: Option<String>,
        #[arg(long)]
        tagline: Option<String>,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        phone: Option<String>,
        /// Bank / payment detail line as "Label: Value". Repeat for each
        /// line. When any --bank-line is passed, REPLACES all existing
        /// bank details on the issuer.
        #[arg(long = "bank-line")]
        bank_line: Vec<String>,
        /// Remove all bank details from the issuer.
        #[arg(long)]
        bank_clear: bool,
        #[arg(long)]
        template: Option<String>,
        #[arg(long)]
        currency: Option<String>,
        #[arg(long)]
        symbol: Option<String>,
        /// Invoice number format. Tokens: {issuer}, {year}, {seq}, {seq:04}.
        /// Use a unique prefix per issuer for globally addressable invoice ids.
        #[arg(long)]
        number_format: Option<String>,
        #[arg(long)]
        logo: Option<String>,
        /// Remove the logo from the issuer (falls back to the star mark).
        #[arg(long)]
        logo_clear: bool,
        /// Default directory for `invoices render` output when --out is
        /// omitted. Leading `~/` is expanded.
        #[arg(long)]
        output_dir: Option<String>,
        /// Default notes auto-populated into new invoices.
        #[arg(long)]
        notes: Option<String>,
    },
    /// Shorthand: change the issuer's default template
    SetTemplate { slug: String, template: String },
    #[command(alias = "ls")]
    List,
    #[command(alias = "get")]
    Show { slug: String },
    #[command(alias = "rm")]
    Delete { slug: String },
}

#[derive(Subcommand, Debug)]
pub enum ClientCmd {
    #[command(alias = "new")]
    Add {
        slug: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        attn: Option<String>,
        #[arg(long)]
        country: Option<String>,
        #[arg(long)]
        tax_id: Option<String>,
        #[arg(long)]
        address: String,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Default issuer slug — `invoices new` uses this when `--as` omitted
        #[arg(long)]
        default_issuer: Option<String>,
        /// Preferred template for this client's invoices
        #[arg(long)]
        default_template: Option<String>,
    },
    /// Edit an existing client — pass only the fields you want to change
    Edit {
        slug: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        attn: Option<String>,
        #[arg(long)]
        country: Option<String>,
        #[arg(long)]
        tax_id: Option<String>,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        default_issuer: Option<String>,
        #[arg(long)]
        default_template: Option<String>,
    },
    /// Shorthand: pin a default issuer for this client
    SetIssuer { slug: String, issuer_slug: String },
    /// Shorthand: pin a preferred template for this client
    SetTemplate { slug: String, template: String },
    #[command(alias = "ls")]
    List,
    #[command(alias = "get")]
    Show { slug: String },
    #[command(alias = "rm")]
    Delete { slug: String },
}

#[derive(Subcommand, Debug)]
pub enum ProductCmd {
    #[command(alias = "new")]
    Add {
        slug: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        subtitle: Option<String>,
        #[arg(long, default_value = "unit")]
        unit: String,
        /// Unit price as a decimal (e.g. 220.00)
        #[arg(long)]
        price: String,
        #[arg(long)]
        currency: String,
        #[arg(long, default_value = "0")]
        tax_rate: String,
    },
    /// Edit an existing product — pass only the fields you want to change
    Edit {
        slug: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        subtitle: Option<String>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        price: Option<String>,
        #[arg(long)]
        currency: Option<String>,
        #[arg(long)]
        tax_rate: Option<String>,
    },
    #[command(alias = "ls")]
    List,
    #[command(alias = "get")]
    Show { slug: String },
    #[command(alias = "rm")]
    Delete { slug: String },
}

#[derive(Subcommand, Debug)]
pub enum InvoiceCmd {
    /// Create a new invoice
    New {
        /// Issuer slug (the "as" — whose invoice is this?). Optional if the
        /// client has a `default_issuer` pinned.
        #[arg(long)]
        r#as: Option<String>,
        /// Client slug
        #[arg(long)]
        client: String,
        /// Item in the form: "product-slug" OR "Description:qty:price:rate"
        #[arg(long = "item")]
        items: Vec<String>,
        /// Due date (e.g. "2026-05-17" or "7d"). Defaults to one week
        /// after issue.
        #[arg(long, default_value = "7d")]
        due: String,
        /// Terms label (default: "Pay in full")
        #[arg(long, default_value = "Pay in full")]
        terms: String,
        #[arg(long)]
        notes: Option<String>,
        /// Currency override (otherwise uses issuer's default)
        #[arg(long)]
        currency: Option<String>,
        /// Reverse-charge flag (EU B2B cross-border)
        #[arg(long)]
        reverse_charge: bool,
        /// Payment URL (Stripe Payment Link, EPC-QR, any URL) encoded as QR
        #[arg(long)]
        pay_link: Option<String>,
        /// Invoice-level discount rate (percent, e.g. "10" for 10% off subtotal)
        #[arg(long)]
        discount_rate: Option<String>,
        /// Invoice-level fixed discount in major units (e.g. "50.00")
        #[arg(long)]
        discount_fixed: Option<String>,
    },
    /// Edit an existing DRAFT invoice's metadata (issued/paid/void invoices
    /// are immutable — use a credit note instead).
    Edit {
        number: String,
        #[arg(long)]
        client: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long)]
        terms: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        currency: Option<String>,
        #[arg(long)]
        pay_link: Option<String>,
        #[arg(long)]
        reverse_charge: Option<bool>,
        #[arg(long)]
        discount_rate: Option<String>,
        #[arg(long)]
        discount_fixed: Option<String>,
    },
    /// Manage line items on a DRAFT invoice
    #[command(subcommand)]
    Items(InvoiceItemCmd),
    /// Issue a credit note against an existing invoice
    CreditNote {
        /// Source invoice number
        number: String,
        /// Copy ALL line items from source and reverse their quantities.
        /// Mutually exclusive with --item.
        #[arg(long, conflicts_with = "items")]
        full: bool,
        /// Explicit items to include on the credit note (same format as
        /// `invoices new --item`). Positive refund amounts are stored as
        /// credits automatically.
        #[arg(long = "item")]
        items: Vec<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        pay_link: Option<String>,
    },
    /// Ageing report for unpaid invoices, bucketed 0-30 / 31-60 / 61-90 / 90+
    Aging {
        #[arg(long = "as")]
        issuer: Option<String>,
    },
    /// Export invoices as CSV / JSON — month-end accountant handoff
    Export {
        /// YYYY-MM-DD inclusive lower bound on issue_date
        #[arg(long)]
        from: Option<String>,
        /// YYYY-MM-DD inclusive upper bound on issue_date
        #[arg(long)]
        to: Option<String>,
        /// csv | json (default csv)
        #[arg(long, default_value = "csv")]
        format: String,
        /// Output path. Defaults to stdout.
        #[arg(long, short)]
        out: Option<String>,
        #[arg(long = "as")]
        issuer: Option<String>,
    },
    /// Clone an existing invoice's line items into a new draft — same client,
    /// new number + dates. Handy for recurring billing.
    Duplicate {
        number: String,
        /// Override the client (defaults to the source invoice's client)
        #[arg(long)]
        client: Option<String>,
        /// Override the issuer (defaults to the source invoice's issuer)
        #[arg(long = "as")]
        r#as: Option<String>,
        /// New due date (e.g. "2026-05-17" or "7d"). Defaults to "7d".
        #[arg(long, default_value = "7d")]
        due: String,
    },
    #[command(alias = "ls")]
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "as")]
        issuer: Option<String>,
        /// Only show invoices past due date and not paid/void
        #[arg(long)]
        overdue: bool,
    },
    #[command(alias = "get")]
    Show { number: String },
    /// Render invoice to PDF
    Render {
        number: String,
        /// Template to use (overrides issuer default)
        #[arg(long)]
        template: Option<String>,
        /// Output path (defaults to ./invoice-<number>.pdf)
        #[arg(long, short)]
        out: Option<String>,
        /// Open the PDF after rendering (macOS open / linux xdg-open)
        #[arg(long)]
        open: bool,
    },
    /// Mark status (draft/issued/paid/void)
    Mark { number: String, status: String },
    #[command(alias = "rm")]
    Delete {
        number: String,
        /// Allow deleting a non-draft invoice. Breaks number-sequence
        /// integrity — prefer `mark void` or credit note in most cases.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum InvoiceItemCmd {
    /// Add a line item to a draft invoice
    Add {
        number: String,
        /// Item spec: "product-slug[:qty]" OR "Description:qty:price[:rate]"
        spec: String,
        #[arg(long)]
        subtitle: Option<String>,
        #[arg(long)]
        discount_rate: Option<String>,
        #[arg(long)]
        discount_fixed: Option<String>,
    },
    /// Remove the item at `position` (zero-indexed) from a draft invoice
    #[command(alias = "rm")]
    Remove { number: String, position: i64 },
    /// Edit the item at `position` — any subset of fields
    Edit {
        number: String,
        position: i64,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        subtitle: Option<String>,
        #[arg(long)]
        qty: Option<String>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        price: Option<String>,
        #[arg(long)]
        tax_rate: Option<String>,
        #[arg(long)]
        discount_rate: Option<String>,
        #[arg(long)]
        discount_fixed: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TemplateCmd {
    #[command(alias = "ls")]
    List,
    /// Render a preview with synthetic data
    Preview {
        name: String,
        #[arg(long, short)]
        out: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    Show,
    Path,
    Set { key: String, value: String },
}

#[derive(Subcommand, Debug)]
pub enum SkillCmd {
    Install,
}
