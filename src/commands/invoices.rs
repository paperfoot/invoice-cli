use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

use crate::cli::InvoiceCmd;
use crate::db::{self, Invoice, InvoiceItem};
use crate::error::{AppError, Result};
use crate::money::MinorUnits;
use crate::output::{print_success, Ctx};
use crate::render;

pub fn run(cmd: InvoiceCmd, ctx: Ctx) -> Result<()> {
    let mut conn = db::open()?;
    match cmd {
        InvoiceCmd::New {
            r#as,
            client,
            items,
            due,
            terms,
            notes,
            currency,
            reverse_charge,
            pay_link,
        } => {
            let client_row = db::client_by_slug(&conn, &client)?;
            let issuer_slug = match r#as {
                Some(s) => s,
                None => client_row.default_issuer_slug.clone().ok_or_else(|| {
                    AppError::InvalidInput(
                        "--as required (no default issuer on this client)".into(),
                    )
                })?,
            };
            let issuer = db::issuer_by_slug(&conn, &issuer_slug)?;
            let profile = issuer.jurisdiction.profile();
            let today = chrono::Local::now().date_naive();
            let due_date = parse_due(&due, today)?;
            let number = db::next_invoice_number(&conn, &issuer, today.year())?;

            let use_currency = currency
                .or(issuer.currency.clone())
                .unwrap_or_else(|| profile.currency.to_string());
            let use_symbol = issuer
                .symbol
                .clone()
                .unwrap_or_else(|| profile.symbol.to_string());

            let parsed_items = parse_items(&conn, &items, &profile)?;

            let invoice = Invoice {
                id: 0,
                number: number.clone(),
                issuer_id: issuer.id,
                client_id: client_row.id,
                issue_date: today.format("%Y-%m-%d").to_string(),
                due_date: due_date.format("%Y-%m-%d").to_string(),
                terms,
                currency: use_currency,
                symbol: use_symbol,
                tax_label: profile.tax_label.to_string(),
                status: "draft".into(),
                notes,
                reverse_charge,
                pay_link,
                issued_at: None,
                paid_at: None,
                total_minor: None,
                items: parsed_items,
            };
            let id = db::invoice_create(&mut conn, &invoice)?;

            let mut out = invoice.clone();
            out.id = id;
            print_success(ctx, &out, |i| {
                println!("created invoice {} (id {})", i.number, i.id)
            });
            Ok(())
        }
        InvoiceCmd::Duplicate {
            number,
            client,
            r#as,
            due,
        } => {
            let source = db::invoice_get(&conn, &number)?;

            // Resolve target client
            let target_client = match client {
                Some(slug) => db::client_by_slug(&conn, &slug)?,
                None => db::client_list(&conn)?
                    .into_iter()
                    .find(|c| c.id == source.client_id)
                    .ok_or_else(|| AppError::NotFound("client".into()))?,
            };

            // Resolve target issuer
            let target_issuer = match r#as {
                Some(slug) => db::issuer_by_slug(&conn, &slug)?,
                None => db::issuer_list(&conn)?
                    .into_iter()
                    .find(|i| i.id == source.issuer_id)
                    .ok_or_else(|| AppError::NotFound("issuer".into()))?,
            };

            let today = chrono::Local::now().date_naive();
            let due_date = parse_due(&due, today)?;
            let new_number = db::next_invoice_number(&conn, &target_issuer, today.year())?;

            let new_items: Vec<InvoiceItem> = source
                .items
                .iter()
                .enumerate()
                .map(|(pos, it)| InvoiceItem {
                    id: 0,
                    invoice_id: 0,
                    position: pos as i64,
                    description: it.description.clone(),
                    subtitle: it.subtitle.clone(),
                    qty: it.qty,
                    unit: it.unit.clone(),
                    unit_price: it.unit_price,
                    tax_rate: it.tax_rate,
                    product_id: it.product_id,
                })
                .collect();

            let invoice = Invoice {
                id: 0,
                number: new_number.clone(),
                issuer_id: target_issuer.id,
                client_id: target_client.id,
                issue_date: today.format("%Y-%m-%d").to_string(),
                due_date: due_date.format("%Y-%m-%d").to_string(),
                terms: source.terms.clone(),
                currency: source.currency.clone(),
                symbol: source.symbol.clone(),
                tax_label: source.tax_label.clone(),
                status: "draft".into(),
                notes: source.notes.clone(),
                reverse_charge: source.reverse_charge,
                pay_link: None,
                issued_at: None,
                paid_at: None,
                total_minor: None,
                items: new_items,
            };
            let id = db::invoice_create(&mut conn, &invoice)?;

            let mut out = invoice.clone();
            out.id = id;
            print_success(ctx, &out, |i| {
                println!("duplicated {} → {} (id {})", number, i.number, i.id)
            });
            Ok(())
        }
        InvoiceCmd::List { status, issuer } => {
            let list = db::invoice_list(&conn, status.as_deref(), issuer.as_deref())?;
            print_success(ctx, &list, |list| {
                if list.is_empty() {
                    println!("no invoices.");
                }
                for i in list {
                    let total = i
                        .total_minor
                        .map(|t| MinorUnits(t).format_with_symbol(&i.symbol))
                        .unwrap_or_else(|| "-".into());
                    println!(
                        "{:<18}  {:<8}  {:<10}  {:<5}  {:>12}",
                        i.number, i.status, i.issue_date, i.currency, total
                    );
                }
            });
            Ok(())
        }
        InvoiceCmd::Show { number } => {
            let inv = db::invoice_get(&conn, &number)?;
            print_success(ctx, &inv, |i| println!("{:#?}", i));
            Ok(())
        }
        InvoiceCmd::Render {
            number,
            template,
            out,
            open,
        } => {
            let inv = db::invoice_get(&conn, &number)?;
            let issuer_rows = db::issuer_list(&conn)?;
            let issuer = issuer_rows
                .into_iter()
                .find(|i| i.id == inv.issuer_id)
                .ok_or_else(|| AppError::NotFound("issuer".into()))?;
            let client = db::client_list(&conn)?
                .into_iter()
                .find(|c| c.id == inv.client_id)
                .ok_or_else(|| AppError::NotFound("client".into()))?;

            // Template resolution chain:
            //   --template CLI arg (if Some)
            //   → client.default_template (if Some AND typst_assets::has_template)
            //   → issuer.default_template (if typst_assets::has_template)
            //   → "vienna"
            let tmpl = template.unwrap_or_else(|| {
                if let Some(ct) = client.default_template.as_ref() {
                    if crate::typst_assets::has_template(ct).unwrap_or(false) {
                        return ct.clone();
                    }
                }
                let stored = issuer.default_template.clone();
                if crate::typst_assets::has_template(&stored).unwrap_or(false) {
                    stored
                } else {
                    "vienna".into()
                }
            });
            let out_path = out
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("invoice-{}.pdf", inv.number)));

            render::render_invoice_with_qr(&tmpl, &inv, &issuer, &client, inv.pay_link.as_deref(), &out_path)?;

            if open {
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&out_path).status();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&out_path).status();
            }

            print_success(
                ctx,
                &serde_json::json!({ "number": inv.number, "path": out_path }),
                |_| println!("rendered → {}", out_path.display()),
            );
            Ok(())
        }
        InvoiceCmd::Mark { number, status } => {
            if !["draft", "issued", "paid", "void"].contains(&status.as_str()) {
                return Err(AppError::InvalidInput(format!(
                    "status must be draft | issued | paid | void, got '{status}'"
                )));
            }
            db::invoice_set_status(&conn, &number, &status)?;
            print_success(ctx, &serde_json::json!({"number": number, "status": status}), |v| {
                println!("marked {} as {}", v["number"], v["status"])
            });
            Ok(())
        }
        InvoiceCmd::Delete { number } => {
            db::invoice_delete(&conn, &number)?;
            print_success(ctx, &number, |n| println!("deleted invoice '{n}'"));
            Ok(())
        }
    }
}

fn parse_due(s: &str, today: NaiveDate) -> Result<NaiveDate> {
    if let Some(stripped) = s.strip_suffix('d') {
        let days: i64 = stripped
            .parse()
            .map_err(|_| AppError::InvalidInput(format!("bad due '{s}'")))?;
        return Ok(today + Duration::days(days));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::InvalidInput(format!("bad due date '{s}': {e}")))
}

fn parse_items(
    conn: &rusqlite::Connection,
    items: &[String],
    profile: &crate::tax::TaxProfile,
) -> Result<Vec<InvoiceItem>> {
    let mut out = Vec::new();
    for (pos, spec) in items.iter().enumerate() {
        // Form 1: product-slug[:qty]
        // Form 2: "Description:qty:price[:rate]"
        let parts: Vec<&str> = spec.split(':').collect();
        let item = match parts.as_slice() {
            [slug] => item_from_product(conn, slug, Decimal::from(1), pos)?,
            [slug, qty] => {
                let q = Decimal::from_str(qty)
                    .map_err(|e| AppError::InvalidInput(format!("bad qty: {e}")))?;
                item_from_product(conn, slug, q, pos)?
            }
            [desc, qty, price] => InvoiceItem {
                id: 0,
                invoice_id: 0,
                position: pos as i64,
                description: desc.to_string(),
                subtitle: None,
                qty: Decimal::from_str(qty)
                    .map_err(|e| AppError::InvalidInput(format!("bad qty: {e}")))?,
                unit: "unit".into(),
                unit_price: MinorUnits::from_decimal(Decimal::from_str(price).map_err(|e| {
                    AppError::InvalidInput(format!("bad price: {e}"))
                })?),
                tax_rate: Decimal::try_from(profile.default_rate).unwrap_or_default(),
                product_id: None,
            },
            [desc, qty, price, rate] => InvoiceItem {
                id: 0,
                invoice_id: 0,
                position: pos as i64,
                description: desc.to_string(),
                subtitle: None,
                qty: Decimal::from_str(qty)
                    .map_err(|e| AppError::InvalidInput(format!("bad qty: {e}")))?,
                unit: "unit".into(),
                unit_price: MinorUnits::from_decimal(Decimal::from_str(price).map_err(|e| {
                    AppError::InvalidInput(format!("bad price: {e}"))
                })?),
                tax_rate: Decimal::from_str(rate)
                    .map_err(|e| AppError::InvalidInput(format!("bad rate: {e}")))?,
                product_id: None,
            },
            _ => {
                return Err(AppError::InvalidInput(format!(
                    "unrecognized item spec '{spec}' — use product-slug[:qty] or description:qty:price[:rate]"
                )))
            }
        };
        out.push(item);
    }
    Ok(out)
}

fn item_from_product(
    conn: &rusqlite::Connection,
    slug: &str,
    qty: Decimal,
    pos: usize,
) -> Result<InvoiceItem> {
    let p = db::product_by_slug(conn, slug)?;
    Ok(InvoiceItem {
        id: 0,
        invoice_id: 0,
        position: pos as i64,
        description: p.description,
        subtitle: p.subtitle,
        qty,
        unit: p.unit,
        unit_price: p.unit_price,
        tax_rate: p.tax_rate,
        product_id: Some(p.id),
    })
}
