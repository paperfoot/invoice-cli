use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

use crate::cli::{InvoiceCmd, InvoiceItemCmd};
use crate::db::{self, Invoice, InvoiceItem};
use crate::error::{AppError, Result};
use crate::money::MinorUnits;
use crate::output::{print_success, Ctx, Format};
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
            discount_rate,
            discount_fixed,
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
            let number = db::next_invoice_number(&conn, &issuer, today.year(), "invoice")?;

            let use_currency = currency
                .or(issuer.currency.clone())
                .unwrap_or_else(|| profile.currency.to_string());
            let use_symbol = issuer
                .symbol
                .clone()
                .unwrap_or_else(|| profile.symbol.to_string());

            let parsed_items = parse_items(&conn, &items, &profile)?;

            let (inv_discount_rate, inv_discount_fixed) =
                parse_discount_pair(discount_rate.as_deref(), discount_fixed.as_deref())?;

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
                kind: "invoice".into(),
                credits_invoice_id: None,
                discount_rate: inv_discount_rate,
                discount_fixed: inv_discount_fixed,
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
        InvoiceCmd::Edit {
            number,
            client,
            due,
            terms,
            notes,
            currency,
            pay_link,
            reverse_charge,
            discount_rate,
            discount_fixed,
        } => {
            let mut inv = db::invoice_get(&conn, &number)?;
            if let Some(slug) = client {
                let c = db::client_by_slug(&conn, &slug)?;
                inv.client_id = c.id;
            }
            if let Some(d) = due {
                let today = chrono::Local::now().date_naive();
                let nd = parse_due(&d, today)?;
                inv.due_date = nd.format("%Y-%m-%d").to_string();
            }
            if let Some(t) = terms {
                inv.terms = t;
            }
            if let Some(n) = notes {
                inv.notes = Some(n);
            }
            if let Some(c) = currency {
                inv.currency = c;
            }
            if let Some(p) = pay_link {
                inv.pay_link = Some(p);
            }
            if let Some(rc) = reverse_charge {
                inv.reverse_charge = rc;
            }
            if discount_rate.is_some() || discount_fixed.is_some() {
                let (dr, df) =
                    parse_discount_pair(discount_rate.as_deref(), discount_fixed.as_deref())?;
                inv.discount_rate = dr;
                inv.discount_fixed = df;
            }
            db::invoice_update_draft(&conn, &inv)?;
            print_success(ctx, &inv, |i| println!("edited invoice {}", i.number));
            Ok(())
        }
        InvoiceCmd::Items(item_cmd) => run_items(item_cmd, ctx, &mut conn),
        InvoiceCmd::CreditNote {
            number,
            full,
            items,
            notes,
            pay_link,
        } => {
            let source = db::invoice_get(&conn, &number)?;
            let issuer = db::issuer_list(&conn)?
                .into_iter()
                .find(|i| i.id == source.issuer_id)
                .ok_or_else(|| AppError::NotFound("issuer".into()))?;
            let profile = issuer.jurisdiction.profile();

            let new_items: Vec<InvoiceItem> = if full {
                source
                    .items
                    .iter()
                    .enumerate()
                    .map(|(pos, it)| InvoiceItem {
                        id: 0,
                        invoice_id: 0,
                        position: pos as i64,
                        description: it.description.clone(),
                        subtitle: it.subtitle.clone(),
                        qty: -it.qty,
                        unit: it.unit.clone(),
                        unit_price: it.unit_price,
                        tax_rate: it.tax_rate,
                        product_id: it.product_id,
                        discount_rate: it.discount_rate,
                        discount_fixed: it.discount_fixed,
                    })
                    .collect()
            } else if !items.is_empty() {
                parse_items(&conn, &items, &profile)?
            } else {
                return Err(AppError::InvalidInput(
                    "pass --full or at least one --item".into(),
                ));
            };

            let today = chrono::Local::now().date_naive();
            let new_number =
                db::next_invoice_number(&conn, &issuer, today.year(), "credit_note")?;

            let invoice = Invoice {
                id: 0,
                number: new_number.clone(),
                issuer_id: source.issuer_id,
                client_id: source.client_id,
                issue_date: today.format("%Y-%m-%d").to_string(),
                due_date: today.format("%Y-%m-%d").to_string(),
                terms: source.terms.clone(),
                currency: source.currency.clone(),
                symbol: source.symbol.clone(),
                tax_label: source.tax_label.clone(),
                status: "draft".into(),
                notes,
                reverse_charge: source.reverse_charge,
                pay_link,
                issued_at: None,
                paid_at: None,
                total_minor: None,
                kind: "credit_note".into(),
                credits_invoice_id: Some(source.id),
                discount_rate: None,
                discount_fixed: None,
                items: new_items,
            };
            let id = db::invoice_create(&mut conn, &invoice)?;

            let mut out = invoice.clone();
            out.id = id;
            print_success(ctx, &out, |i| {
                println!(
                    "created credit note {} (against {}, id {})",
                    i.number, number, i.id
                )
            });
            Ok(())
        }
        InvoiceCmd::Aging { issuer } => {
            let list = db::invoice_list(&conn, Some("issued"), issuer.as_deref())?;
            let today = chrono::Local::now().date_naive();

            #[derive(Default, serde::Serialize)]
            struct Bucket {
                count: i64,
                total_minor: i64,
            }
            let mut buckets: std::collections::BTreeMap<&'static str, Bucket> =
                Default::default();
            for name in ["not_yet_due", "0_30", "31_60", "61_90", "90_plus"] {
                buckets.insert(name, Bucket::default());
            }

            #[derive(serde::Serialize)]
            struct Row {
                number: String,
                client_id: i64,
                due_date: String,
                days_overdue: i64,
                total_minor: i64,
            }
            let mut rows: Vec<Row> = Vec::new();
            for inv in &list {
                let due = NaiveDate::parse_from_str(&inv.due_date, "%Y-%m-%d")
                    .unwrap_or(today);
                let days_overdue = (today - due).num_days();
                let bucket_name: &'static str = if days_overdue <= 0 {
                    "not_yet_due"
                } else if days_overdue <= 30 {
                    "0_30"
                } else if days_overdue <= 60 {
                    "31_60"
                } else if days_overdue <= 90 {
                    "61_90"
                } else {
                    "90_plus"
                };
                let total = inv.total_minor.unwrap_or(0);
                let b = buckets.get_mut(bucket_name).unwrap();
                b.count += 1;
                b.total_minor += total;
                rows.push(Row {
                    number: inv.number.clone(),
                    client_id: inv.client_id,
                    due_date: inv.due_date.clone(),
                    days_overdue,
                    total_minor: total,
                });
            }

            let symbol = list
                .first()
                .map(|i| i.symbol.clone())
                .unwrap_or_else(|| "$".into());

            let payload = serde_json::json!({
                "buckets": &buckets,
                "invoices": &rows,
            });

            print_success(ctx, &payload, |_| {
                println!(
                    "{:<14} {:>6} {:>14}",
                    "bucket", "count", "total"
                );
                for (name, b) in &buckets {
                    let total = MinorUnits(b.total_minor).format_with_symbol(&symbol);
                    println!("{:<14} {:>6} {:>14}", name, b.count, total);
                }
            });
            Ok(())
        }
        InvoiceCmd::Export {
            from,
            to,
            format,
            out,
            issuer,
        } => {
            let list = db::invoice_list(&conn, None, issuer.as_deref())?;

            let from_date = from
                .as_deref()
                .map(|s| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                        AppError::InvalidInput(format!("bad --from '{s}': {e}"))
                    })
                })
                .transpose()?;
            let to_date = to
                .as_deref()
                .map(|s| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                        AppError::InvalidInput(format!("bad --to '{s}': {e}"))
                    })
                })
                .transpose()?;

            let filtered: Vec<Invoice> = list
                .into_iter()
                .filter(|inv| {
                    let d = match NaiveDate::parse_from_str(&inv.issue_date, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => return false,
                    };
                    if let Some(f) = from_date {
                        if d < f {
                            return false;
                        }
                    }
                    if let Some(t) = to_date {
                        if d > t {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            match format.as_str() {
                "csv" => {
                    // Build a lookup for issuer_id -> slug, client_id -> slug
                    let issuers = db::issuer_list(&conn)?;
                    let clients = db::client_list(&conn)?;
                    let issuer_slug = |id: i64| -> String {
                        issuers
                            .iter()
                            .find(|x| x.id == id)
                            .map(|x| x.slug.clone())
                            .unwrap_or_default()
                    };
                    let client_slug = |id: i64| -> String {
                        clients
                            .iter()
                            .find(|x| x.id == id)
                            .map(|x| x.slug.clone())
                            .unwrap_or_default()
                    };

                    let mut csv = String::new();
                    csv.push_str("number,kind,issue_date,due_date,status,issued_at,paid_at,currency,total_minor,issuer_slug,client_slug,notes\n");
                    for inv in &filtered {
                        let row = [
                            csv_esc(&inv.number),
                            csv_esc(&inv.kind),
                            csv_esc(&inv.issue_date),
                            csv_esc(&inv.due_date),
                            csv_esc(&inv.status),
                            csv_esc(inv.issued_at.as_deref().unwrap_or("")),
                            csv_esc(inv.paid_at.as_deref().unwrap_or("")),
                            csv_esc(&inv.currency),
                            inv.total_minor.map(|t| t.to_string()).unwrap_or_default(),
                            csv_esc(&issuer_slug(inv.issuer_id)),
                            csv_esc(&client_slug(inv.client_id)),
                            csv_esc(inv.notes.as_deref().unwrap_or("")),
                        ];
                        csv.push_str(&row.join(","));
                        csv.push('\n');
                    }

                    if let Some(path) = out {
                        std::fs::write(&path, csv)?;
                        print_success(
                            ctx,
                            &serde_json::json!({"path": path, "count": filtered.len()}),
                            |v| println!("exported {} invoices → {}", v["count"], v["path"]),
                        );
                    } else {
                        // No out path: if JSON envelope active, wrap as data;
                        // otherwise emit raw CSV to stdout.
                        match ctx.format {
                            Format::Json => {
                                print_success(
                                    ctx,
                                    &serde_json::json!({"csv": csv, "count": filtered.len()}),
                                    |_| {},
                                );
                            }
                            Format::Human => {
                                print!("{}", csv);
                            }
                        }
                    }
                }
                "json" => {
                    let serialized = serde_json::to_string_pretty(&filtered)?;
                    if let Some(path) = out {
                        std::fs::write(&path, &serialized)?;
                        print_success(
                            ctx,
                            &serde_json::json!({"path": path, "count": filtered.len()}),
                            |v| println!("exported {} invoices → {}", v["count"], v["path"]),
                        );
                    } else {
                        print_success(ctx, &filtered, |list| {
                            println!("{}", serde_json::to_string_pretty(list).unwrap());
                        });
                    }
                }
                other => {
                    return Err(AppError::InvalidInput(format!(
                        "unknown export format '{other}' — use csv or json"
                    )));
                }
            }
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
            let new_number =
                db::next_invoice_number(&conn, &target_issuer, today.year(), "invoice")?;

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
                    discount_rate: it.discount_rate,
                    discount_fixed: it.discount_fixed,
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
                kind: "invoice".into(),
                credits_invoice_id: None,
                discount_rate: None,
                discount_fixed: None,
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
        InvoiceCmd::List { status, issuer, overdue } => {
            let mut list = db::invoice_list(&conn, status.as_deref(), issuer.as_deref())?;
            if overdue {
                let today = chrono::Local::now().date_naive();
                list.retain(|inv| {
                    if inv.status == "paid" || inv.status == "void" {
                        return false;
                    }
                    match NaiveDate::parse_from_str(&inv.due_date, "%Y-%m-%d") {
                        Ok(d) => d < today,
                        Err(_) => false,
                    }
                });
            }
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
        InvoiceCmd::Delete { number, force } => {
            db::invoice_delete(&conn, &number, force)?;
            print_success(ctx, &number, |n| println!("deleted invoice '{n}'"));
            Ok(())
        }
    }
}

fn run_items(cmd: InvoiceItemCmd, ctx: Ctx, conn: &mut rusqlite::Connection) -> Result<()> {
    match cmd {
        InvoiceItemCmd::Add {
            number,
            spec,
            subtitle,
            discount_rate,
            discount_fixed,
        } => {
            let inv = db::invoice_get(conn, &number)?;
            let issuer = db::issuer_list(conn)?
                .into_iter()
                .find(|i| i.id == inv.issuer_id)
                .ok_or_else(|| AppError::NotFound("issuer".into()))?;
            let profile = issuer.jurisdiction.profile();
            let specs = [spec.clone()];
            let mut parsed = parse_items(conn, &specs, &profile)?;
            let mut item = parsed
                .pop()
                .ok_or_else(|| AppError::InvalidInput(format!("no item parsed from '{spec}'")))?;
            if let Some(st) = subtitle {
                item.subtitle = Some(st);
            }
            let (dr, df) =
                parse_discount_pair(discount_rate.as_deref(), discount_fixed.as_deref())?;
            item.discount_rate = dr;
            item.discount_fixed = df;

            let item_id = db::invoice_item_add(conn, inv.id, &item)?;
            print_success(
                ctx,
                &serde_json::json!({
                    "invoice": inv.number,
                    "item_id": item_id,
                }),
                |v| {
                    println!(
                        "added item to {} (item id {})",
                        v["invoice"], v["item_id"]
                    )
                },
            );
            Ok(())
        }
        InvoiceItemCmd::Remove { number, position } => {
            let inv = db::invoice_get(conn, &number)?;
            db::invoice_item_remove(conn, inv.id, position)?;
            print_success(
                ctx,
                &serde_json::json!({"invoice": inv.number, "position": position}),
                |v| println!("removed item {} from {}", v["position"], v["invoice"]),
            );
            Ok(())
        }
        InvoiceItemCmd::Edit {
            number,
            position,
            description,
            subtitle,
            qty,
            unit,
            price,
            tax_rate,
            discount_rate,
            discount_fixed,
        } => {
            let inv = db::invoice_get(conn, &number)?;
            let mut item = inv
                .items
                .iter()
                .find(|it| it.position == position)
                .cloned()
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "item at position {position} of invoice '{}'",
                        inv.number
                    ))
                })?;

            if let Some(d) = description {
                item.description = d;
            }
            if let Some(s) = subtitle {
                item.subtitle = Some(s);
            }
            if let Some(q) = qty {
                item.qty = Decimal::from_str(&q)
                    .map_err(|e| AppError::InvalidInput(format!("bad qty: {e}")))?;
            }
            if let Some(u) = unit {
                item.unit = u;
            }
            if let Some(p) = price {
                let d = Decimal::from_str(&p)
                    .map_err(|e| AppError::InvalidInput(format!("bad price: {e}")))?;
                item.unit_price = MinorUnits::from_decimal(d);
            }
            if let Some(r) = tax_rate {
                item.tax_rate = Decimal::from_str(&r)
                    .map_err(|e| AppError::InvalidInput(format!("bad tax rate: {e}")))?;
            }
            if discount_rate.is_some() || discount_fixed.is_some() {
                let (dr, df) =
                    parse_discount_pair(discount_rate.as_deref(), discount_fixed.as_deref())?;
                item.discount_rate = dr;
                item.discount_fixed = df;
            }

            db::invoice_item_edit(conn, inv.id, position, &item)?;
            print_success(
                ctx,
                &serde_json::json!({"invoice": inv.number, "position": position}),
                |v| println!("edited item {} of {}", v["position"], v["invoice"]),
            );
            Ok(())
        }
    }
}

/// Parse at most one of (--discount-rate, --discount-fixed). Returns a pair
/// where at most one Option is Some.
fn parse_discount_pair(
    rate: Option<&str>,
    fixed: Option<&str>,
) -> Result<(Option<Decimal>, Option<MinorUnits>)> {
    match (rate, fixed) {
        (Some(_), Some(_)) => Err(AppError::InvalidInput(
            "pass at most one of --discount-rate / --discount-fixed".into(),
        )),
        (Some(r), None) => {
            let d = Decimal::from_str(r)
                .map_err(|e| AppError::InvalidInput(format!("bad discount rate: {e}")))?;
            Ok((Some(d), None))
        }
        (None, Some(f)) => {
            let d = Decimal::from_str(f)
                .map_err(|e| AppError::InvalidInput(format!("bad discount fixed: {e}")))?;
            Ok((None, Some(MinorUnits::from_decimal(d))))
        }
        (None, None) => Ok((None, None)),
    }
}

/// Basic CSV field escaping per RFC 4180: if field contains comma, quote or
/// newline, wrap in double quotes and double up any interior quotes.
fn csv_esc(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
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
                discount_rate: None,
                discount_fixed: None,
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
                discount_rate: None,
                discount_fixed: None,
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
        discount_rate: None,
        discount_fixed: None,
    })
}
