// ═══════════════════════════════════════════════════════════════════════════
// Database layer — invoice-cli queries over the shared finance-core SQLite.
//
// Connection opening, the refinery migration runner, and the `Issuer`
// primitive all live in finance-core so the whole accounting suite shares
// one schema and one DB file. This file owns the invoice-specific queries
// (clients, products, invoices, invoice_items, number_series).
// ═══════════════════════════════════════════════════════════════════════════

use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

use crate::error::{AppError, Result};
use crate::money::MinorUnits;
use crate::tax::Jurisdiction;

pub use finance_core::entity::Issuer;

pub fn open() -> Result<Connection> {
    let paths = finance_core::paths::Paths::resolve()?;
    Ok(finance_core::db::open(&paths)?)
}

pub fn open_at(path: &Path) -> Result<Connection> {
    Ok(finance_core::db::open_at(path)?)
}

// ─── Domain types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub attn: Option<String>,
    pub country: Option<String>,
    pub tax_id: Option<String>,
    pub address: Vec<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    /// If set, `invoices new` defaults `--as` to this issuer slug when omitted.
    pub default_issuer_slug: Option<String>,
    /// If set, render uses this template before falling back to the issuer's
    /// default_template. Explicit `--template` CLI flag still wins.
    pub default_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub slug: String,
    pub description: String,
    pub subtitle: Option<String>,
    pub unit: String,
    pub unit_price: MinorUnits,
    pub currency: String,
    pub tax_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub number: String,
    pub issuer_id: i64,
    pub client_id: i64,
    pub issue_date: String,
    pub due_date: String,
    pub terms: String,
    pub currency: String,
    pub symbol: String,
    pub tax_label: String,
    pub status: String,
    pub notes: Option<String>,
    pub reverse_charge: bool,
    /// Optional URL (Stripe Payment Link, EPC-QR payload, any URL) that the
    /// renderer encodes as a QR code on the PDF.
    pub pay_link: Option<String>,
    /// Timestamp (ISO-8601) when the invoice was first marked 'issued'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<String>,
    /// Timestamp (ISO-8601) when the invoice was first marked 'paid'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    /// Grand total in minor units. Only populated by `invoice_list`, not by
    /// `invoice_get` (which returns all items so callers can sum themselves).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_minor: Option<i64>,
    /// "invoice" or "credit_note". Credit notes reference a source invoice
    /// via `credits_invoice_id` and are typically displayed with a "CN-"
    /// prefix and "CREDIT NOTE" title.
    pub kind: String,
    pub credits_invoice_id: Option<i64>,
    /// Invoice-level discount rate (percent, as decimal string e.g. "10").
    /// Applied to pre-tax subtotal after line-level discounts.
    pub discount_rate: Option<Decimal>,
    /// Invoice-level fixed discount in minor units.
    pub discount_fixed: Option<MinorUnits>,
    pub items: Vec<InvoiceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub id: i64,
    pub invoice_id: i64,
    pub position: i64,
    pub description: String,
    pub subtitle: Option<String>,
    pub qty: Decimal,
    pub unit: String,
    pub unit_price: MinorUnits,
    pub tax_rate: Decimal,
    pub product_id: Option<i64>,
    /// Per-line discount rate (percent, as Decimal e.g. 10 for 10%). Applied
    /// to (qty * unit_price) pre-tax. Mutually exclusive with discount_fixed.
    pub discount_rate: Option<Decimal>,
    /// Per-line fixed discount in minor units. Applied to line pre-tax.
    pub discount_fixed: Option<MinorUnits>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn addr_to_text(lines: &[String]) -> String {
    lines.join("\n")
}
fn text_to_addr(s: &str) -> Vec<String> {
    s.split('\n').map(|l| l.to_string()).collect()
}

// ─── Issuers ─────────────────────────────────────────────────────────────

pub fn issuer_create(conn: &Connection, issuer: &Issuer) -> Result<i64> {
    conn.execute(
        "INSERT INTO issuers (slug, name, legal_name, jurisdiction, tax_registered,
                              tax_id, company_no, tagline, address, email, phone,
                              bank_details, default_template,
                              currency, symbol, number_format, logo_path,
                              default_output_dir, default_notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            issuer.slug,
            issuer.name,
            issuer.legal_name,
            issuer.jurisdiction.as_str(),
            issuer.tax_registered as i32,
            issuer.tax_id,
            issuer.company_no,
            issuer.tagline,
            addr_to_text(&issuer.address),
            issuer.email,
            issuer.phone,
            issuer.bank_details,
            issuer.default_template,
            issuer.currency,
            issuer.symbol,
            issuer.number_format,
            issuer.logo_path,
            issuer.default_output_dir,
            issuer.default_notes,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn issuer_list(conn: &Connection) -> Result<Vec<Issuer>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, name, legal_name, jurisdiction, tax_registered,
                tax_id, company_no, tagline, address, email, phone,
                bank_details, default_template,
                currency, symbol, number_format, logo_path,
                default_output_dir, default_notes
         FROM issuers ORDER BY slug",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Issuer {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                legal_name: row.get(3)?,
                jurisdiction: Jurisdiction::from_str(&row.get::<_, String>(4)?)
                    .unwrap_or(Jurisdiction::Custom),
                tax_registered: row.get::<_, i32>(5)? != 0,
                tax_id: row.get(6)?,
                company_no: row.get(7)?,
                tagline: row.get(8)?,
                address: text_to_addr(&row.get::<_, String>(9)?),
                email: row.get(10)?,
                phone: row.get(11)?,
                bank_details: row.get(12)?,
                default_template: row.get(13)?,
                currency: row.get(14)?,
                symbol: row.get(15)?,
                number_format: row.get(16)?,
                logo_path: row.get(17)?,
                default_output_dir: row.get(18)?,
                default_notes: row.get(19)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn issuer_by_slug(conn: &Connection, slug: &str) -> Result<Issuer> {
    // Exact match first
    for i in issuer_list(conn)? {
        if i.slug == slug {
            return Ok(i);
        }
    }
    // Fuzzy fallback (substring on slug or case-insensitive contains on name)
    let lower = slug.to_lowercase();
    let matches: Vec<Issuer> = issuer_list(conn)?
        .into_iter()
        .filter(|i| i.slug.contains(slug) || i.name.to_lowercase().contains(&lower))
        .collect();
    match matches.len() {
        0 => Err(AppError::NotFound(format!("issuer '{slug}'"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(AppError::Ambiguous(format!(
            "issuer '{slug}' matches {}",
            matches
                .iter()
                .map(|m| m.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn issuer_delete(conn: &Connection, slug: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM issuers WHERE slug = ?1", params![slug])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("issuer '{slug}'")));
    }
    Ok(())
}

/// Full-replace UPDATE. Matches by slug (PK-like). Slug itself cannot change.
pub fn issuer_update(conn: &Connection, issuer: &Issuer) -> Result<()> {
    let affected = conn.execute(
        "UPDATE issuers SET
             name = ?1, legal_name = ?2, jurisdiction = ?3, tax_registered = ?4,
             tax_id = ?5, company_no = ?6, tagline = ?7, address = ?8,
             email = ?9, phone = ?10, bank_details = ?11, default_template = ?12,
             currency = ?13, symbol = ?14, number_format = ?15, logo_path = ?16,
             default_output_dir = ?17, default_notes = ?18
         WHERE slug = ?19",
        params![
            issuer.name,
            issuer.legal_name,
            issuer.jurisdiction.as_str(),
            issuer.tax_registered as i32,
            issuer.tax_id,
            issuer.company_no,
            issuer.tagline,
            addr_to_text(&issuer.address),
            issuer.email,
            issuer.phone,
            issuer.bank_details,
            issuer.default_template,
            issuer.currency,
            issuer.symbol,
            issuer.number_format,
            issuer.logo_path,
            issuer.default_output_dir,
            issuer.default_notes,
            issuer.slug,
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("issuer '{}'", issuer.slug)));
    }
    Ok(())
}

// ─── Clients ─────────────────────────────────────────────────────────────

pub fn client_create(conn: &Connection, client: &Client) -> Result<i64> {
    conn.execute(
        "INSERT INTO clients (slug, name, attn, country, tax_id, address, email, notes,
                              default_issuer_slug, default_template)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            client.slug,
            client.name,
            client.attn,
            client.country,
            client.tax_id,
            addr_to_text(&client.address),
            client.email,
            client.notes,
            client.default_issuer_slug,
            client.default_template,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn client_list(conn: &Connection) -> Result<Vec<Client>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, name, attn, country, tax_id, address, email, notes,
                default_issuer_slug, default_template
         FROM clients ORDER BY slug",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Client {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                attn: row.get(3)?,
                country: row.get(4)?,
                tax_id: row.get(5)?,
                address: text_to_addr(&row.get::<_, String>(6)?),
                email: row.get(7)?,
                notes: row.get(8)?,
                default_issuer_slug: row.get(9)?,
                default_template: row.get(10)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn client_by_slug(conn: &Connection, slug: &str) -> Result<Client> {
    for c in client_list(conn)? {
        if c.slug == slug {
            return Ok(c);
        }
    }
    // Try fuzzy match
    let matches: Vec<Client> = client_list(conn)?
        .into_iter()
        .filter(|c| c.slug.contains(slug) || c.name.to_lowercase().contains(&slug.to_lowercase()))
        .collect();
    match matches.len() {
        0 => Err(AppError::NotFound(format!("client '{slug}'"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(AppError::Ambiguous(format!(
            "client '{slug}' matches {}",
            matches
                .iter()
                .map(|m| m.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn client_delete(conn: &Connection, slug: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM clients WHERE slug = ?1", params![slug])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("client '{slug}'")));
    }
    Ok(())
}

/// Full-replace UPDATE. Matches by slug.
pub fn client_update(conn: &Connection, client: &Client) -> Result<()> {
    let affected = conn.execute(
        "UPDATE clients SET
             name = ?1, attn = ?2, country = ?3, tax_id = ?4, address = ?5,
             email = ?6, notes = ?7, default_issuer_slug = ?8, default_template = ?9
         WHERE slug = ?10",
        params![
            client.name,
            client.attn,
            client.country,
            client.tax_id,
            addr_to_text(&client.address),
            client.email,
            client.notes,
            client.default_issuer_slug,
            client.default_template,
            client.slug,
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("client '{}'", client.slug)));
    }
    Ok(())
}

// ─── Products ────────────────────────────────────────────────────────────

pub fn product_create(conn: &Connection, p: &Product) -> Result<i64> {
    conn.execute(
        "INSERT INTO products (slug, description, subtitle, unit, unit_price_minor, currency, tax_rate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            p.slug,
            p.description,
            p.subtitle,
            p.unit,
            p.unit_price.0,
            p.currency,
            p.tax_rate.to_string(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn product_list(conn: &Connection) -> Result<Vec<Product>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, description, subtitle, unit, unit_price_minor, currency, tax_rate
         FROM products ORDER BY slug",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Product {
                id: row.get(0)?,
                slug: row.get(1)?,
                description: row.get(2)?,
                subtitle: row.get(3)?,
                unit: row.get(4)?,
                unit_price: MinorUnits(row.get::<_, i64>(5)?),
                currency: row.get(6)?,
                tax_rate: Decimal::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn product_by_slug(conn: &Connection, slug: &str) -> Result<Product> {
    for p in product_list(conn)? {
        if p.slug == slug {
            return Ok(p);
        }
    }
    let matches: Vec<Product> = product_list(conn)?
        .into_iter()
        .filter(|p| p.slug.contains(slug))
        .collect();
    match matches.len() {
        0 => Err(AppError::NotFound(format!("product '{slug}'"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(AppError::Ambiguous(format!(
            "product '{slug}' matches {}",
            matches
                .iter()
                .map(|m| m.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn product_delete(conn: &Connection, slug: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM products WHERE slug = ?1", params![slug])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("product '{slug}'")));
    }
    Ok(())
}

/// Full-replace UPDATE. Matches by slug.
pub fn product_update(conn: &Connection, product: &Product) -> Result<()> {
    let affected = conn.execute(
        "UPDATE products SET
             description = ?1, subtitle = ?2, unit = ?3, unit_price_minor = ?4,
             currency = ?5, tax_rate = ?6
         WHERE slug = ?7",
        params![
            product.description,
            product.subtitle,
            product.unit,
            product.unit_price.0,
            product.currency,
            product.tax_rate.to_string(),
            product.slug,
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("product '{}'", product.slug)));
    }
    Ok(())
}

// ─── Invoices ────────────────────────────────────────────────────────────

/// Generate the next document number for an issuer/year/kind combination.
/// `kind` is typically "invoice" or "credit_note" — credit notes get an
/// independent sequence and are formatted with a "CN-" prefix.
pub fn next_invoice_number(
    conn: &Connection,
    issuer: &Issuer,
    year: i32,
    kind: &str,
) -> Result<String> {
    let seq: i64 = conn.query_row(
        "INSERT INTO number_series (issuer_id, year, kind, next_seq)
         VALUES (?1, ?2, ?3, 2)
         ON CONFLICT(issuer_id, year, kind) DO UPDATE SET next_seq = next_seq + 1
         RETURNING next_seq - 1",
        params![issuer.id, year, kind],
        |r| r.get(0),
    )?;

    let out = format_document_number(issuer, year, seq, kind);
    ensure_globally_unique_number(conn, issuer, &out)
}

fn format_document_number(issuer: &Issuer, year: i32, seq: i64, kind: &str) -> String {
    let mut out = issuer.number_format.clone();
    out = out.replace("{issuer}", &issuer.slug);
    out = out.replace("{year}", &year.to_string());
    out = apply_sequence_format(&out, seq);
    if kind == "credit_note" {
        out = format!("CN-{out}");
    }
    out
}

fn ensure_globally_unique_number(
    conn: &Connection,
    issuer: &Issuer,
    candidate: &str,
) -> Result<String> {
    if !invoice_number_exists(conn, candidate)? {
        return Ok(candidate.to_string());
    }

    let prefixed = issuer_prefixed_number(&issuer.slug, candidate);
    if !invoice_number_exists(conn, &prefixed)? {
        return Ok(prefixed);
    }

    Err(AppError::InvalidInput(format!(
        "generated invoice number '{candidate}' already exists globally. Set a unique issuer number format, e.g. `invoice issuer edit {} --number-format \"{}-{{year}}-{{seq:04}}\"`",
        issuer.slug, issuer.slug
    )))
}

fn invoice_number_exists(conn: &Connection, number: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM invoices WHERE number = ?1 LIMIT 1",
            params![number],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn issuer_prefixed_number(issuer_slug: &str, candidate: &str) -> String {
    if let Some(rest) = candidate.strip_prefix("CN-") {
        format!("CN-{issuer_slug}-{rest}")
    } else {
        format!("{issuer_slug}-{candidate}")
    }
}

fn apply_sequence_format(format: &str, seq: i64) -> String {
    if let Some(start) = format.find("{seq:") {
        let width_start = start + "{seq:".len();
        if let Some(relative_end) = format[width_start..].find('}') {
            let end = width_start + relative_end;
            if let Ok(width) = format[width_start..end].parse::<usize>() {
                let token = &format[start..=end];
                return format.replace(token, &format!("{:0width$}", seq, width = width));
            }
        }
    }

    format.replace("{seq}", &seq.to_string())
}

pub fn invoice_create(conn: &mut Connection, inv: &Invoice) -> Result<i64> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO invoices (number, issuer_id, client_id, issue_date, due_date,
                               terms, currency, symbol, tax_label, status, notes,
                               reverse_charge, pay_link, issued_at, paid_at,
                               kind, credits_invoice_id, discount_rate, discount_fixed_minor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            inv.number,
            inv.issuer_id,
            inv.client_id,
            inv.issue_date,
            inv.due_date,
            inv.terms,
            inv.currency,
            inv.symbol,
            inv.tax_label,
            inv.status,
            inv.notes,
            inv.reverse_charge as i32,
            inv.pay_link,
            inv.issued_at,
            inv.paid_at,
            inv.kind,
            inv.credits_invoice_id,
            inv.discount_rate.as_ref().map(|d| d.to_string()),
            inv.discount_fixed.as_ref().map(|m| m.0),
        ],
    )?;
    let id = tx.last_insert_rowid();
    for (pos, item) in inv.items.iter().enumerate() {
        tx.execute(
            "INSERT INTO invoice_items (invoice_id, position, description, subtitle,
                                        qty, unit, unit_price_minor, tax_rate, product_id,
                                        discount_rate, discount_fixed_minor)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                pos as i64,
                item.description,
                item.subtitle,
                item.qty.to_string(),
                item.unit,
                item.unit_price.0,
                item.tax_rate.to_string(),
                item.product_id,
                item.discount_rate.as_ref().map(|d| d.to_string()),
                item.discount_fixed.as_ref().map(|m| m.0),
            ],
        )?;
    }
    tx.commit()?;
    Ok(id)
}

pub fn invoice_get(conn: &Connection, number: &str) -> Result<Invoice> {
    let mut inv: Invoice = conn.query_row(
        "SELECT id, number, issuer_id, client_id, issue_date, due_date, terms,
                currency, symbol, tax_label, status, notes, reverse_charge, pay_link,
                issued_at, paid_at, kind, credits_invoice_id,
                discount_rate, discount_fixed_minor
         FROM invoices WHERE number = ?1",
        params![number],
        |row| {
            Ok(Invoice {
                id: row.get(0)?,
                number: row.get(1)?,
                issuer_id: row.get(2)?,
                client_id: row.get(3)?,
                issue_date: row.get(4)?,
                due_date: row.get(5)?,
                terms: row.get(6)?,
                currency: row.get(7)?,
                symbol: row.get(8)?,
                tax_label: row.get(9)?,
                status: row.get(10)?,
                notes: row.get(11)?,
                reverse_charge: row.get::<_, i32>(12)? != 0,
                pay_link: row.get(13)?,
                issued_at: row.get(14)?,
                paid_at: row.get(15)?,
                kind: row.get(16)?,
                credits_invoice_id: row.get(17)?,
                discount_rate: row
                    .get::<_, Option<String>>(18)?
                    .and_then(|s| Decimal::from_str(&s).ok()),
                discount_fixed: row.get::<_, Option<i64>>(19)?.map(MinorUnits),
                total_minor: None,
                items: vec![],
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, invoice_id, position, description, subtitle, qty, unit,
                unit_price_minor, tax_rate, product_id, discount_rate, discount_fixed_minor
         FROM invoice_items WHERE invoice_id = ?1 ORDER BY position",
    )?;
    let items = stmt
        .query_map(params![inv.id], |row| {
            Ok(InvoiceItem {
                id: row.get(0)?,
                invoice_id: row.get(1)?,
                position: row.get(2)?,
                description: row.get(3)?,
                subtitle: row.get(4)?,
                qty: Decimal::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                unit: row.get(6)?,
                unit_price: MinorUnits(row.get::<_, i64>(7)?),
                tax_rate: Decimal::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                product_id: row.get(9)?,
                discount_rate: row
                    .get::<_, Option<String>>(10)?
                    .and_then(|s| Decimal::from_str(&s).ok()),
                discount_fixed: row.get::<_, Option<i64>>(11)?.map(MinorUnits),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    inv.items = items;
    Ok(inv)
}

pub fn invoice_list(
    conn: &Connection,
    status: Option<&str>,
    issuer_slug: Option<&str>,
) -> Result<Vec<Invoice>> {
    let mut query = String::from(
        "SELECT i.id, i.number, i.issuer_id, i.client_id, i.issue_date, i.due_date,
                i.terms, i.currency, i.symbol, i.tax_label, i.status, i.notes,
                i.reverse_charge, i.pay_link, i.issued_at, i.paid_at,
                i.kind, i.credits_invoice_id, i.discount_rate, i.discount_fixed_minor
         FROM invoices i JOIN issuers s ON s.id = i.issuer_id WHERE 1=1",
    );
    let mut p: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(st) = status {
        query.push_str(" AND i.status = ?");
        p.push(Box::new(st.to_string()));
    }
    if let Some(sl) = issuer_slug {
        query.push_str(" AND s.slug = ?");
        p.push(Box::new(sl.to_string()));
    }
    query.push_str(" ORDER BY i.issue_date DESC");
    let mut stmt = conn.prepare(&query)?;
    let mut rows: Vec<Invoice> = stmt
        .query_map(
            rusqlite::params_from_iter(p.iter().map(|b| b.as_ref())),
            |row| {
                Ok(Invoice {
                    id: row.get(0)?,
                    number: row.get(1)?,
                    issuer_id: row.get(2)?,
                    client_id: row.get(3)?,
                    issue_date: row.get(4)?,
                    due_date: row.get(5)?,
                    terms: row.get(6)?,
                    currency: row.get(7)?,
                    symbol: row.get(8)?,
                    tax_label: row.get(9)?,
                    status: row.get(10)?,
                    notes: row.get(11)?,
                    reverse_charge: row.get::<_, i32>(12)? != 0,
                    pay_link: row.get(13)?,
                    issued_at: row.get(14)?,
                    paid_at: row.get(15)?,
                    kind: row.get(16)?,
                    credits_invoice_id: row.get(17)?,
                    discount_rate: row
                        .get::<_, Option<String>>(18)?
                        .and_then(|s| Decimal::from_str(&s).ok()),
                    discount_fixed: row.get::<_, Option<i64>>(19)?.map(MinorUnits),
                    total_minor: None,
                    items: vec![],
                })
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Populate `total_minor` per invoice. Discount-aware and tax-aware:
    // keep the same proportional invoice-level discount math used by the
    // renderer so `invoices list` and PDFs agree.
    if !rows.is_empty() {
        use crate::money::{line_total_discounted, tax_amount, MinorUnits};
        let mut items_stmt = conn.prepare(
            "SELECT invoice_id, qty, unit_price_minor, tax_rate,
                    discount_rate, discount_fixed_minor
             FROM invoice_items
             WHERE invoice_id IN (SELECT id FROM invoices)",
        )?;
        #[derive(Default)]
        struct Acc {
            subtotal: i64,
            by_rate: std::collections::BTreeMap<String, i64>,
        }
        let mut acc: std::collections::HashMap<i64, Acc> = std::collections::HashMap::new();
        let item_rows = items_stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        })?;
        for r in item_rows {
            let (iid, qty_s, up_minor, rate_s, disc_rate, disc_fixed) = r?;
            let qty = Decimal::from_str(&qty_s).unwrap_or_default();
            let rate = Decimal::from_str(&rate_s).unwrap_or_default();
            let dr = disc_rate.and_then(|s| Decimal::from_str(&s).ok());
            let df = disc_fixed.map(MinorUnits);
            let line = line_total_discounted(qty, MinorUnits(up_minor), dr, df);
            let e = acc.entry(iid).or_default();
            e.subtotal += line.0;
            *e.by_rate.entry(rate.to_string()).or_insert(0) += line.0;
        }
        for inv in rows.iter_mut() {
            if let Some(a) = acc.get(&inv.id) {
                let discount =
                    invoice_discount_minor(a.subtotal, inv.discount_rate, inv.discount_fixed);
                inv.total_minor = Some(total_minor_from_bases(
                    a.subtotal,
                    &a.by_rate,
                    discount,
                    |base, rate| tax_amount(MinorUnits(base), rate).0,
                ));
            }
        }
    }
    Ok(rows)
}

fn invoice_discount_minor(
    subtotal: i64,
    discount_rate: Option<Decimal>,
    discount_fixed: Option<MinorUnits>,
) -> i64 {
    match (discount_rate, discount_fixed) {
        (Some(rate), _) => crate::money::apply_rate(MinorUnits(subtotal), rate)
            .0
            .clamp(0, subtotal),
        (None, Some(fixed)) => fixed.0.clamp(0, subtotal),
        _ => 0,
    }
}

fn total_minor_from_bases<F>(
    subtotal: i64,
    by_rate: &std::collections::BTreeMap<String, i64>,
    discount: i64,
    tax_fn: F,
) -> i64
where
    F: Fn(i64, Decimal) -> i64,
{
    let subtotal_after_discount = subtotal - discount;
    let mut tax_total = 0;
    for (rate_str, base) in by_rate {
        let rate = Decimal::from_str(rate_str).unwrap_or_default();
        let scaled_base = if subtotal > 0 && discount > 0 {
            ((*base as i128) * (subtotal_after_discount as i128) / (subtotal as i128)) as i64
        } else {
            *base
        };
        tax_total += tax_fn(scaled_base, rate);
    }
    subtotal_after_discount + tax_total
}

/// Update status and, on first transition into `issued` / `paid`, stamp the
/// corresponding timestamp column. Idempotent — re-marking doesn't overwrite
/// the original timestamp.
pub fn invoice_set_status(conn: &Connection, number: &str, status: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = match status {
        "issued" => conn.execute(
            "UPDATE invoices
                SET status = ?1,
                    issued_at = COALESCE(issued_at, ?2)
              WHERE number = ?3",
            params![status, now, number],
        )?,
        "paid" => conn.execute(
            "UPDATE invoices
                SET status = ?1,
                    paid_at = COALESCE(paid_at, ?2)
              WHERE number = ?3",
            params![status, now, number],
        )?,
        _ => conn.execute(
            "UPDATE invoices SET status = ?1 WHERE number = ?2",
            params![status, number],
        )?,
    };
    if affected == 0 {
        return Err(AppError::NotFound(format!("invoice '{number}'")));
    }
    Ok(())
}

/// Delete an invoice by number. Refuses non-draft invoices unless `force`.
/// Deleting a non-draft invoice breaks the numbering sequence — which is a
/// regulatory problem in many jurisdictions. Forcing should be deliberate.
pub fn invoice_delete(conn: &Connection, number: &str, force: bool) -> Result<()> {
    let status: Option<String> = conn
        .query_row(
            "SELECT status FROM invoices WHERE number = ?1",
            params![number],
            |r| r.get(0),
        )
        .optional()?;
    let status = status.ok_or_else(|| AppError::NotFound(format!("invoice '{number}'")))?;
    if status != "draft" && !force {
        return Err(AppError::InvalidInput(format!(
            "refusing to delete non-draft invoice '{number}' (status='{status}') — pass --force to override. Prefer voiding or issuing a credit note."
        )));
    }
    conn.execute("DELETE FROM invoices WHERE number = ?1", params![number])?;
    Ok(())
}

/// Draft-only metadata edit. Rejects edits to issued / paid / void invoices —
/// the correct path for those is a credit note.
pub fn invoice_update_draft(conn: &Connection, inv: &Invoice) -> Result<()> {
    let status: Option<String> = conn
        .query_row(
            "SELECT status FROM invoices WHERE number = ?1",
            params![inv.number],
            |r| r.get(0),
        )
        .optional()?;
    let status = status.ok_or_else(|| AppError::NotFound(format!("invoice '{}'", inv.number)))?;
    if status != "draft" {
        return Err(AppError::InvalidInput(format!(
            "invoice '{}' is {status}, not draft — issued invoices are immutable. Use a credit note to correct.",
            inv.number
        )));
    }
    conn.execute(
        "UPDATE invoices SET
             client_id = ?1, issue_date = ?2, due_date = ?3, terms = ?4,
             currency = ?5, symbol = ?6, tax_label = ?7, notes = ?8,
             reverse_charge = ?9, pay_link = ?10,
             discount_rate = ?11, discount_fixed_minor = ?12
         WHERE number = ?13",
        params![
            inv.client_id,
            inv.issue_date,
            inv.due_date,
            inv.terms,
            inv.currency,
            inv.symbol,
            inv.tax_label,
            inv.notes,
            inv.reverse_charge as i32,
            inv.pay_link,
            inv.discount_rate.as_ref().map(|d| d.to_string()),
            inv.discount_fixed.as_ref().map(|m| m.0),
            inv.number,
        ],
    )?;
    Ok(())
}

fn require_draft(conn: &Connection, invoice_id: i64) -> Result<()> {
    let status: String = conn.query_row(
        "SELECT status FROM invoices WHERE id = ?1",
        params![invoice_id],
        |r| r.get(0),
    )?;
    if status != "draft" {
        return Err(AppError::InvalidInput(format!(
            "invoice is {status}, not draft — items cannot be modified. Use a credit note to correct."
        )));
    }
    Ok(())
}

/// Append an item to a draft invoice. Fails if the invoice isn't draft.
pub fn invoice_item_add(conn: &Connection, invoice_id: i64, item: &InvoiceItem) -> Result<i64> {
    require_draft(conn, invoice_id)?;
    let next_pos: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM invoice_items WHERE invoice_id = ?1",
            params![invoice_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO invoice_items (invoice_id, position, description, subtitle,
                                    qty, unit, unit_price_minor, tax_rate, product_id,
                                    discount_rate, discount_fixed_minor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            invoice_id,
            next_pos,
            item.description,
            item.subtitle,
            item.qty.to_string(),
            item.unit,
            item.unit_price.0,
            item.tax_rate.to_string(),
            item.product_id,
            item.discount_rate.as_ref().map(|d| d.to_string()),
            item.discount_fixed.as_ref().map(|m| m.0),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Remove the item at `position` from a draft invoice; re-compacts trailing
/// positions so there are no holes.
pub fn invoice_item_remove(conn: &mut Connection, invoice_id: i64, position: i64) -> Result<()> {
    require_draft(conn, invoice_id)?;
    let tx = conn.transaction()?;
    let affected = tx.execute(
        "DELETE FROM invoice_items WHERE invoice_id = ?1 AND position = ?2",
        params![invoice_id, position],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "item at position {position} of invoice id {invoice_id}"
        )));
    }
    tx.execute(
        "UPDATE invoice_items SET position = position - 1
           WHERE invoice_id = ?1 AND position > ?2",
        params![invoice_id, position],
    )?;
    tx.commit()?;
    Ok(())
}

/// Replace the item at `position` with `item`'s fields. Draft-only.
pub fn invoice_item_edit(
    conn: &Connection,
    invoice_id: i64,
    position: i64,
    item: &InvoiceItem,
) -> Result<()> {
    require_draft(conn, invoice_id)?;
    let affected = conn.execute(
        "UPDATE invoice_items SET
             description = ?1, subtitle = ?2, qty = ?3, unit = ?4,
             unit_price_minor = ?5, tax_rate = ?6, product_id = ?7,
             discount_rate = ?8, discount_fixed_minor = ?9
         WHERE invoice_id = ?10 AND position = ?11",
        params![
            item.description,
            item.subtitle,
            item.qty.to_string(),
            item.unit,
            item.unit_price.0,
            item.tax_rate.to_string(),
            item.product_id,
            item.discount_rate.as_ref().map(|d| d.to_string()),
            item.discount_fixed.as_ref().map(|m| m.0),
            invoice_id,
            position,
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "item at position {position} of invoice id {invoice_id}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::MinorUnits;
    use rust_decimal::Decimal;
    use std::collections::BTreeMap;
    use std::str::FromStr;

    #[test]
    fn applies_variable_width_sequence_tokens() {
        assert_eq!(
            apply_sequence_format("199-AP-{year}-{seq:03}", 2),
            "199-AP-{year}-002"
        );
        assert_eq!(apply_sequence_format("{year}-{seq:04}", 42), "{year}-0042");
        assert_eq!(apply_sequence_format("{year}-{seq}", 7), "{year}-7");
    }

    #[test]
    fn supports_issuer_token_in_number_format() {
        let issuer = test_issuer("paperfoot", "{issuer}-{year}-{seq:03}");
        assert_eq!(
            format_document_number(&issuer, 2026, 12, "invoice"),
            "paperfoot-2026-012"
        );
    }

    #[test]
    fn prefixes_legacy_colliding_number_for_second_issuer() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut conn = open_at(tmp.path()).unwrap();

        let mut alpha = test_issuer("alpha", "{year}-{seq:04}");
        alpha.id = issuer_create(&conn, &alpha).unwrap();
        let mut beta = test_issuer("beta", "{year}-{seq:04}");
        beta.id = issuer_create(&conn, &beta).unwrap();

        let client_id = client_create(&conn, &test_client()).unwrap();
        let first = next_invoice_number(&conn, &alpha, 2026, "invoice").unwrap();
        assert_eq!(first, "2026-0001");
        let inv = test_invoice(first, alpha.id, client_id);
        invoice_create(&mut conn, &inv).unwrap();

        let second = next_invoice_number(&conn, &beta, 2026, "invoice").unwrap();
        assert_eq!(second, "beta-2026-0001");
    }

    #[test]
    fn list_total_scales_tax_base_after_invoice_discount() {
        let mut bases = BTreeMap::new();
        bases.insert("20".to_string(), 10_000);
        let discount = invoice_discount_minor(10_000, Some(Decimal::from_str("50").unwrap()), None);
        let total = total_minor_from_bases(10_000, &bases, discount, |base, rate| {
            crate::money::tax_amount(MinorUnits(base), rate).0
        });
        assert_eq!(total, 6_000);
    }

    fn test_issuer(slug: &str, number_format: &str) -> Issuer {
        Issuer {
            id: 0,
            slug: slug.to_string(),
            name: slug.to_string(),
            legal_name: None,
            jurisdiction: Jurisdiction::Uk,
            tax_registered: false,
            tax_id: None,
            company_no: None,
            tagline: None,
            address: vec!["1 Test Street".into()],
            email: None,
            phone: None,
            bank_details: None,
            default_template: "vienna".into(),
            currency: Some("GBP".into()),
            symbol: Some("£".into()),
            number_format: number_format.into(),
            logo_path: None,
            default_output_dir: None,
            default_notes: None,
        }
    }

    fn test_client() -> Client {
        Client {
            id: 0,
            slug: "client".into(),
            name: "Client".into(),
            attn: None,
            country: None,
            tax_id: None,
            address: vec!["1 Client Street".into()],
            email: None,
            notes: None,
            default_issuer_slug: None,
            default_template: None,
        }
    }

    fn test_invoice(number: String, issuer_id: i64, client_id: i64) -> Invoice {
        Invoice {
            id: 0,
            number,
            issuer_id,
            client_id,
            issue_date: "2026-01-01".into(),
            due_date: "2026-01-08".into(),
            terms: "Pay in full".into(),
            currency: "GBP".into(),
            symbol: "£".into(),
            tax_label: "VAT".into(),
            status: "draft".into(),
            notes: None,
            reverse_charge: false,
            pay_link: None,
            issued_at: None,
            paid_at: None,
            total_minor: None,
            kind: "invoice".into(),
            credits_invoice_id: None,
            discount_rate: None,
            discount_fixed: None,
            items: vec![],
        }
    }
}
