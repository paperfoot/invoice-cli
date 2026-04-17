use crate::cli::ClientCmd;
use crate::db::{self, Client};
use crate::error::{AppError, Result};
use crate::output::{print_success, Ctx};

pub fn run(cmd: ClientCmd, ctx: Ctx) -> Result<()> {
    let conn = db::open()?;
    match cmd {
        ClientCmd::Add {
            slug,
            name,
            attn,
            country,
            tax_id,
            address,
            email,
            notes,
            default_issuer,
            default_template,
        } => {
            if let Some(ref iss) = default_issuer {
                db::issuer_by_slug(&conn, iss).map_err(|_| {
                    AppError::InvalidInput(format!("unknown issuer '{iss}'"))
                })?;
            }
            if let Some(ref tmpl) = default_template {
                if !crate::typst_assets::has_template(tmpl)? {
                    let available = crate::typst_assets::list_templates()?.join(", ");
                    return Err(AppError::InvalidInput(format!(
                        "unknown template '{tmpl}' — available: {available}"
                    )));
                }
            }
            let client = Client {
                id: 0,
                slug: slug.clone(),
                name,
                attn,
                country,
                tax_id,
                address: address.split('\n').map(|s| s.to_string()).collect(),
                email,
                notes,
                default_issuer_slug: default_issuer,
                default_template,
            };
            let id = db::client_create(&conn, &client)?;
            let mut out = client.clone();
            out.id = id;
            print_success(ctx, &out, |c| println!("added client '{}' (id {})", c.slug, c.id));
            Ok(())
        }
        ClientCmd::Edit {
            slug,
            name,
            attn,
            country,
            tax_id,
            address,
            email,
            notes,
            default_issuer,
            default_template,
        } => {
            if let Some(ref iss) = default_issuer {
                db::issuer_by_slug(&conn, iss).map_err(|_| {
                    AppError::InvalidInput(format!("unknown issuer '{iss}'"))
                })?;
            }
            if let Some(ref tmpl) = default_template {
                if !crate::typst_assets::has_template(tmpl)? {
                    let available = crate::typst_assets::list_templates()?.join(", ");
                    return Err(AppError::InvalidInput(format!(
                        "unknown template '{tmpl}' — available: {available}"
                    )));
                }
            }
            let mut client = db::client_by_slug(&conn, &slug)?;
            if let Some(v) = name {
                client.name = v;
            }
            if let Some(v) = attn {
                client.attn = Some(v);
            }
            if let Some(v) = country {
                client.country = Some(v);
            }
            if let Some(v) = tax_id {
                client.tax_id = Some(v);
            }
            if let Some(v) = address {
                client.address = v.split('\n').map(|s| s.to_string()).collect();
            }
            if let Some(v) = email {
                client.email = Some(v);
            }
            if let Some(v) = notes {
                client.notes = Some(v);
            }
            if let Some(v) = default_issuer {
                client.default_issuer_slug = Some(v);
            }
            if let Some(v) = default_template {
                client.default_template = Some(v);
            }
            db::client_update(&conn, &client)?;
            print_success(ctx, &client, |c| {
                println!("updated client '{}' (id {})", c.slug, c.id)
            });
            Ok(())
        }
        ClientCmd::SetIssuer { slug, issuer_slug } => {
            db::issuer_by_slug(&conn, &issuer_slug).map_err(|_| {
                AppError::InvalidInput(format!("unknown issuer '{issuer_slug}'"))
            })?;
            let mut client = db::client_by_slug(&conn, &slug)?;
            client.default_issuer_slug = Some(issuer_slug);
            db::client_update(&conn, &client)?;
            print_success(ctx, &client, |c| {
                println!(
                    "set default issuer for client '{}' to '{}'",
                    c.slug,
                    c.default_issuer_slug.as_deref().unwrap_or("-")
                )
            });
            Ok(())
        }
        ClientCmd::SetTemplate { slug, template } => {
            if !crate::typst_assets::has_template(&template)? {
                let available = crate::typst_assets::list_templates()?.join(", ");
                return Err(AppError::InvalidInput(format!(
                    "unknown template '{template}' — available: {available}"
                )));
            }
            let mut client = db::client_by_slug(&conn, &slug)?;
            client.default_template = Some(template);
            db::client_update(&conn, &client)?;
            print_success(ctx, &client, |c| {
                println!(
                    "set default template for client '{}' to '{}'",
                    c.slug,
                    c.default_template.as_deref().unwrap_or("-")
                )
            });
            Ok(())
        }
        ClientCmd::List => {
            let list = db::client_list(&conn)?;
            print_success(ctx, &list, |list| {
                if list.is_empty() {
                    println!("no clients. add one: invoice clients add <slug> --name ...");
                }
                for c in list {
                    let defaults = match (&c.default_issuer_slug, &c.default_template) {
                        (None, None) => String::new(),
                        (iss, tmpl) => {
                            let mut parts: Vec<String> = Vec::new();
                            if let Some(i) = iss {
                                parts.push(format!("issuer:{i}"));
                            }
                            if let Some(t) = tmpl {
                                parts.push(format!("tmpl:{t}"));
                            }
                            format!("  [{}]", parts.join(", "))
                        }
                    };
                    println!(
                        "{:<16}  {:<32}  {}{}",
                        c.slug,
                        c.name,
                        c.country.as_deref().unwrap_or("-"),
                        defaults
                    );
                }
            });
            Ok(())
        }
        ClientCmd::Show { slug } => {
            let c = db::client_by_slug(&conn, &slug)?;
            print_success(ctx, &c, |c| println!("{:#?}", c));
            Ok(())
        }
        ClientCmd::Delete { slug } => {
            db::client_delete(&conn, &slug)?;
            print_success(ctx, &slug, |s| println!("deleted client '{s}'"));
            Ok(())
        }
    }
}
