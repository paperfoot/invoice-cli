use crate::cli::IssuerCmd;
use crate::commands::split_multiline_arg;
use crate::db::{self, Issuer};
use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};
use crate::tax::Jurisdiction;

pub fn run(cmd: IssuerCmd, ctx: Ctx) -> Result<()> {
    let conn = db::open()?;
    match cmd {
        IssuerCmd::Add {
            slug,
            name,
            legal_name,
            jurisdiction,
            tax_registered,
            tax_id,
            company_no,
            address,
            email,
            phone,
            bank_line,
            template,
            logo,
            output_dir,
            notes,
        } => {
            let jur = Jurisdiction::from_str(&jurisdiction).ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "unknown jurisdiction '{jurisdiction}' — use one of: sg, uk, us, eu, custom"
                ))
            })?;
            let profile = jur.profile();
            let issuer = Issuer {
                id: 0,
                slug,
                name,
                legal_name,
                jurisdiction: jur,
                tax_registered,
                tax_id,
                company_no,
                tagline: None,
                address: split_multiline_arg(&address),
                email,
                phone,
                bank_details: if bank_line.is_empty() {
                    None
                } else {
                    Some(bank_line.join("\n"))
                },
                default_template: template,
                currency: Some(profile.currency.to_string()),
                symbol: Some(profile.symbol.to_string()),
                number_format: "{year}-{seq:04}".into(),
                logo_path: logo,
                default_output_dir: output_dir,
                default_notes: notes,
            };
            let id = db::issuer_create(&conn, &issuer)?;
            let mut out = issuer.clone();
            out.id = id;
            print_success(ctx, &out, |i| {
                println!("added issuer '{}' (id {})", i.slug, i.id)
            });
            Ok(())
        }
        IssuerCmd::List => {
            let list = db::issuer_list(&conn)?;
            print_success(ctx, &list, |list| {
                if list.is_empty() {
                    println!("no issuers. add one: invoice issuer add <slug> --name ...");
                }
                for i in list {
                    println!(
                        "{:<16}  {:<24}  {} ({})",
                        i.slug,
                        i.name,
                        i.jurisdiction.as_str(),
                        if i.tax_registered {
                            "tax-registered"
                        } else {
                            "-"
                        }
                    );
                }
            });
            Ok(())
        }
        IssuerCmd::Show { slug } => {
            let i = db::issuer_by_slug(&conn, &slug)?;
            print_success(ctx, &i, |i| println!("{:#?}", i));
            Ok(())
        }
        IssuerCmd::Delete { slug } => {
            db::issuer_delete(&conn, &slug)?;
            print_success(ctx, &slug, |s| println!("deleted issuer '{s}'"));
            Ok(())
        }
        IssuerCmd::Edit {
            slug,
            name,
            legal_name,
            jurisdiction,
            tax_registered,
            tax_id,
            company_no,
            tagline,
            address,
            email,
            phone,
            bank_line,
            bank_clear,
            template,
            output_dir,
            notes,
            currency,
            symbol,
            number_format,
            logo,
            logo_clear,
        } => {
            let mut issuer = db::issuer_by_slug(&conn, &slug)?;
            if let Some(v) = name {
                issuer.name = v;
            }
            if let Some(v) = legal_name {
                issuer.legal_name = Some(v);
            }
            if let Some(v) = jurisdiction {
                let jur = Jurisdiction::from_str(&v).ok_or_else(|| {
                    AppError::InvalidInput(format!(
                        "unknown jurisdiction '{v}' — use one of: sg, uk, us, eu, custom"
                    ))
                })?;
                issuer.jurisdiction = jur;
            }
            if let Some(v) = tax_registered {
                issuer.tax_registered = v;
            }
            if let Some(v) = tax_id {
                issuer.tax_id = Some(v);
            }
            if let Some(v) = company_no {
                issuer.company_no = Some(v);
            }
            if let Some(v) = tagline {
                issuer.tagline = Some(v);
            }
            if let Some(v) = address {
                issuer.address = split_multiline_arg(&v);
            }
            if let Some(v) = email {
                issuer.email = Some(v);
            }
            if let Some(v) = phone {
                issuer.phone = Some(v);
            }
            if bank_clear {
                issuer.bank_details = None;
            } else if !bank_line.is_empty() {
                issuer.bank_details = Some(bank_line.join("\n"));
            }
            if let Some(v) = template {
                issuer.default_template = v;
            }
            if let Some(v) = output_dir {
                issuer.default_output_dir = Some(v);
            }
            if let Some(v) = notes {
                issuer.default_notes = Some(v);
            }
            // Currency + symbol auto-linking: if user sets currency but
            // doesn't supply a symbol, derive the conventional one via
            // finance_core::money::currency_symbol. An explicit --symbol
            // still wins when both are given.
            let currency_changed = currency.is_some();
            if let Some(v) = currency {
                if !v.is_empty() {
                    let derived = finance_core::money::currency_symbol(&v);
                    if symbol.is_none() && !derived.is_empty() {
                        issuer.symbol = Some(derived.to_string());
                    }
                    issuer.currency = Some(v);
                }
            }
            if let Some(v) = symbol {
                issuer.symbol = Some(v);
            } else if !currency_changed {
                // no-op: neither currency nor symbol changed
            }
            if let Some(v) = number_format {
                issuer.number_format = v;
            }
            if logo_clear {
                issuer.logo_path = None;
            } else if let Some(v) = logo {
                issuer.logo_path = Some(v);
            }
            db::issuer_update(&conn, &issuer)?;
            print_success(ctx, &issuer, |i| {
                println!("updated issuer '{}' (id {})", i.slug, i.id)
            });
            Ok(())
        }
        IssuerCmd::SetTemplate { slug, template } => {
            if !crate::typst_assets::has_template(&template)? {
                let available = crate::typst_assets::list_templates()?.join(", ");
                return Err(AppError::InvalidInput(format!(
                    "unknown template '{template}' — available: {available}"
                )));
            }
            let mut issuer = db::issuer_by_slug(&conn, &slug)?;
            issuer.default_template = template;
            db::issuer_update(&conn, &issuer)?;
            print_success(ctx, &issuer, |i| {
                println!(
                    "set template for issuer '{}' to '{}'",
                    i.slug, i.default_template
                )
            });
            Ok(())
        }
    }
}
