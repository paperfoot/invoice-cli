// ═══════════════════════════════════════════════════════════════════════════
// Render — generates shared/invoice.typ from DB data + static helpers,
// then shells out to the `typst` binary to produce the PDF.
// ═══════════════════════════════════════════════════════════════════════════

use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use crate::db::{Client, Invoice, Issuer};
use crate::error::{AppError, Result};
use crate::money::{line_total, tax_amount, MinorUnits};
use crate::typst_assets;

#[derive(Debug, Serialize)]
pub struct InvoiceData {
    pub issuer: IssuerData,
    pub client: ClientData,
    pub invoice: InvoiceMeta,
    pub items: Vec<ItemData>,
    pub totals: TotalsData,
    pub notes: String,
    /// Optional QR code matrix (boolean grid) generated from `invoice.pay_link`
    /// or any other `qr_data`. `None` means no QR rendered for this invoice.
    pub qr: Option<QrData>,
}

#[derive(Debug, Serialize)]
pub struct QrData {
    pub modules: Vec<Vec<bool>>,
    pub size: u32,      // module count per side
    pub label: String,  // shown below the code ("Pay online", "Scan to pay", etc.)
}

#[derive(Debug, Serialize)]
pub struct IssuerData {
    pub name: String,
    pub legal_name: Option<String>,
    pub tagline: Option<String>,
    pub address: Vec<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub company_no: Option<String>,
    pub bank: Option<BankData>,
}

#[derive(Debug, Serialize)]
pub struct BankData {
    pub name: String,
    pub iban: String,
    pub bic: String,
}

#[derive(Debug, Serialize)]
pub struct ClientData {
    pub name: String,
    pub attn: Option<String>,
    pub address: Vec<String>,
    pub tax_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceMeta {
    pub number: String,
    pub issue_date: String,
    pub due_date: String,
    pub terms: String,
    pub currency: String,
    pub symbol: String,
    pub tax_label: String,
    pub title: String,
    pub reverse_charge: bool,
}

#[derive(Debug, Serialize)]
pub struct ItemData {
    pub description: String,
    pub subtitle: Option<String>,
    pub qty: f64,
    pub unit: String,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct TotalsData {
    pub subtotal: f64,
    pub tax_lines: Vec<TaxLine>,
    pub tax_total: f64,
    pub total: f64,
}

#[derive(Debug, Serialize)]
pub struct TaxLine {
    pub rate: f64,
    pub base: f64,
    pub amount: f64,
}

pub fn build_data(inv: &Invoice, issuer: &Issuer, client: &Client) -> InvoiceData {
    let profile = issuer.jurisdiction.profile();
    let title = profile.title(issuer.tax_registered).to_string();

    let mut items = Vec::with_capacity(inv.items.len());
    let mut subtotal = MinorUnits(0);
    let mut by_rate: std::collections::BTreeMap<String, MinorUnits> = Default::default();

    for it in &inv.items {
        let line = line_total(it.qty, it.unit_price);
        subtotal.0 += line.0;
        let k = it.tax_rate.to_string();
        let entry = by_rate.entry(k).or_insert(MinorUnits(0));
        entry.0 += line.0;

        items.push(ItemData {
            description: it.description.clone(),
            subtitle: it.subtitle.clone(),
            qty: it.qty.to_f64().unwrap_or(0.0),
            unit: it.unit.clone(),
            unit_price: it.unit_price.as_major(),
            tax_rate: it.tax_rate.to_f64().unwrap_or(0.0),
            amount: line.as_major(),
        });
    }

    let mut tax_lines = Vec::new();
    let mut tax_total = MinorUnits(0);
    for (rate_str, base) in &by_rate {
        let rate = Decimal::from_str(rate_str).unwrap_or_default();
        let amt = tax_amount(*base, rate);
        tax_total.0 += amt.0;
        tax_lines.push(TaxLine {
            rate: rate.to_f64().unwrap_or(0.0),
            base: base.as_major(),
            amount: amt.as_major(),
        });
    }

    InvoiceData {
        issuer: IssuerData {
            name: issuer.name.clone(),
            legal_name: issuer.legal_name.clone(),
            tagline: issuer.tagline.clone(),
            address: issuer.address.clone(),
            email: issuer.email.clone(),
            phone: issuer.phone.clone(),
            tax_id: issuer.tax_id.clone(),
            company_no: issuer.company_no.clone(),
            bank: issuer.bank_name.as_ref().and_then(|n| {
                Some(BankData {
                    name: n.clone(),
                    iban: issuer.bank_iban.clone()?,
                    bic: issuer.bank_bic.clone()?,
                })
            }),
        },
        client: ClientData {
            name: client.name.clone(),
            attn: client.attn.clone(),
            address: client.address.clone(),
            tax_id: client.tax_id.clone(),
        },
        invoice: InvoiceMeta {
            number: inv.number.clone(),
            issue_date: format_date(&inv.issue_date, profile.date_format),
            due_date: format_date(&inv.due_date, profile.date_format),
            terms: inv.terms.clone(),
            currency: inv.currency.clone(),
            symbol: inv.symbol.clone(),
            tax_label: inv.tax_label.clone(),
            title,
            reverse_charge: inv.reverse_charge,
        },
        items,
        totals: TotalsData {
            subtotal: subtotal.as_major(),
            tax_lines,
            tax_total: tax_total.as_major(),
            total: MinorUnits(subtotal.0 + tax_total.0).as_major(),
        },
        notes: inv.notes.clone().unwrap_or_default(),
        qr: None, // Caller can set via build_data_with_qr below
    }
}

/// Encode an arbitrary string (URL, EPC-QR payload, plain text) into a QR
/// module matrix suitable for Typst rendering. Medium error-correction level
/// (Q) — robust while keeping module count low for clean look.
pub fn encode_qr(data: &str) -> Option<QrData> {
    if data.is_empty() {
        return None;
    }
    let code = qrcode::QrCode::with_error_correction_level(
        data.as_bytes(),
        qrcode::EcLevel::Q,
    )
    .ok()?;
    let width = code.width();
    let colors = code.to_colors();
    let modules: Vec<Vec<bool>> = (0..width)
        .map(|row| {
            (0..width)
                .map(|col| {
                    matches!(
                        colors[row * width + col],
                        qrcode::Color::Dark
                    )
                })
                .collect()
        })
        .collect();
    Some(QrData {
        modules,
        size: width as u32,
        label: "Pay online".to_string(),
    })
}

/// Convenience: build invoice data and attach a QR from `qr_data` if present.
pub fn build_data_with_qr(
    inv: &Invoice,
    issuer: &Issuer,
    client: &Client,
    qr_data: Option<&str>,
) -> InvoiceData {
    let mut data = build_data(inv, issuer, client);
    data.qr = qr_data.and_then(encode_qr);
    data
}

pub fn render_invoice(
    template: &str,
    inv: &Invoice,
    issuer: &Issuer,
    client: &Client,
    out_path: &Path,
) -> Result<()> {
    render_invoice_with_qr(template, inv, issuer, client, None, out_path)
}

pub fn render_invoice_with_qr(
    template: &str,
    inv: &Invoice,
    issuer: &Issuer,
    client: &Client,
    qr_data: Option<&str>,
    out_path: &Path,
) -> Result<()> {
    typst_assets::ensure_extracted()?;
    if !typst_assets::has_template(template)? {
        return Err(AppError::InvalidInput(format!(
            "template '{template}' not found. Run: invoice template list"
        )));
    }

    let data = build_data_with_qr(inv, issuer, client, qr_data);
    inject_sample_data(&data)?;

    let template_path = typst_assets::template_path(template)?;
    let root = typst_assets::project_root()?;

    let status = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(&root)
        .arg(&template_path)
        .arg(out_path)
        .status()
        .map_err(|e| AppError::Render(format!("typst binary not found: {e}")))?;

    if !status.success() {
        return Err(AppError::Render(format!(
            "typst compile exited with {}",
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}

fn inject_sample_data(data: &InvoiceData) -> Result<()> {
    let shared = typst_assets::shared_dir()?;
    let invoice_path = shared.join("invoice.typ");
    let helpers_path = shared.join("_helpers.inc.typ");
    let helpers = std::fs::read_to_string(&helpers_path)
        .map_err(|e| AppError::Render(format!("missing _helpers.inc.typ: {e}")))?;

    let sample = generate_sample_data_typ(data);
    let full = format!(
        "// Auto-generated by invoice-cli. Manual edits will be overwritten.\n\n{}\n\n{}",
        sample, helpers
    );
    std::fs::write(&invoice_path, full)?;
    Ok(())
}

fn generate_sample_data_typ(d: &InvoiceData) -> String {
    format!(
        "#let sample-data = (\n  issuer: {},\n  client: {},\n  invoice: {},\n  items: {},\n  notes: {},\n  qr: {},\n)",
        typst_dict_issuer(&d.issuer),
        typst_dict_client(&d.client),
        typst_dict_invoice(&d.invoice),
        typst_array_items(&d.items),
        typst_string(&d.notes),
        typst_qr(&d.qr),
    )
}

fn typst_qr(qr: &Option<QrData>) -> String {
    match qr {
        None => "none".into(),
        Some(q) => {
            let rows: Vec<String> = q
                .modules
                .iter()
                .map(|row| {
                    let cells: Vec<&str> = row
                        .iter()
                        .map(|&b| if b { "true" } else { "false" })
                        .collect();
                    format!("({})", cells.join(", "))
                })
                .collect();
            format!(
                "(modules: ({}), size: {}, label: {})",
                rows.join(", "),
                q.size,
                typst_string(&q.label),
            )
        }
    }
}

fn typst_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn typst_opt(s: &Option<String>) -> String {
    match s {
        Some(v) => typst_string(v),
        None => "none".into(),
    }
}

fn typst_array(lines: &[String]) -> String {
    let items: Vec<String> = lines.iter().map(|l| typst_string(l)).collect();
    format!("({})", items.join(", "))
}

fn typst_dict_issuer(i: &IssuerData) -> String {
    let bank = match &i.bank {
        Some(b) => format!(
            "(name: {}, iban: {}, bic: {})",
            typst_string(&b.name),
            typst_string(&b.iban),
            typst_string(&b.bic)
        ),
        None => "none".into(),
    };
    format!(
        "(name: {}, legal-name: {}, tagline: {}, address: {}, email: {}, phone: {}, tax-id: {}, company-no: {}, bank: {})",
        typst_string(&i.name),
        typst_opt(&i.legal_name),
        typst_opt(&i.tagline),
        typst_array(&i.address),
        typst_opt(&i.email),
        typst_opt(&i.phone),
        typst_opt(&i.tax_id),
        typst_opt(&i.company_no),
        bank,
    )
}

fn typst_dict_client(c: &ClientData) -> String {
    format!(
        "(name: {}, attn: {}, address: {}, tax-id: {})",
        typst_string(&c.name),
        typst_opt(&c.attn),
        typst_array(&c.address),
        typst_opt(&c.tax_id),
    )
}

fn typst_dict_invoice(m: &InvoiceMeta) -> String {
    format!(
        "(number: {}, issue-date: {}, due-date: {}, terms: {}, currency: {}, symbol: {}, tax-label: {}, title: {}, reverse-charge: {})",
        typst_string(&m.number),
        typst_string(&m.issue_date),
        typst_string(&m.due_date),
        typst_string(&m.terms),
        typst_string(&m.currency),
        typst_string(&m.symbol),
        typst_string(&m.tax_label),
        typst_string(&m.title),
        if m.reverse_charge { "true" } else { "false" },
    )
}

/// Convert ISO 8601 date to the jurisdiction's display format.
/// Falls back to the original string if parsing fails.
fn format_date(iso: &str, fmt: &str) -> String {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d")
        .map(|d| d.format(fmt).to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn typst_array_items(items: &[ItemData]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|it| {
            format!(
                "(description: {}, subtitle: {}, qty: {}, unit: {}, unit-price: {}, tax-rate: {})",
                typst_string(&it.description),
                typst_opt(&it.subtitle),
                it.qty,
                typst_string(&it.unit),
                it.unit_price,
                it.tax_rate,
            )
        })
        .collect();
    format!("(\n    {},\n  )", parts.join(",\n    "))
}
