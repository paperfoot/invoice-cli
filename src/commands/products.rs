use rust_decimal::Decimal;
use std::str::FromStr;

use crate::cli::ProductCmd;
use crate::db::{self, Product};
use crate::error::{AppError, Result};
use crate::money::MinorUnits;
use crate::output::{print_success, Ctx};

pub fn run(cmd: ProductCmd, ctx: Ctx) -> Result<()> {
    let conn = db::open()?;
    match cmd {
        ProductCmd::Add {
            slug,
            description,
            subtitle,
            unit,
            price,
            currency,
            tax_rate,
        } => {
            let price_dec = Decimal::from_str(&price)
                .map_err(|e| AppError::InvalidInput(format!("bad price: {e}")))?;
            let rate = Decimal::from_str(&tax_rate)
                .map_err(|e| AppError::InvalidInput(format!("bad tax rate: {e}")))?;
            let product = Product {
                id: 0,
                slug: slug.clone(),
                description,
                subtitle,
                unit,
                unit_price: MinorUnits::from_decimal(price_dec),
                currency,
                tax_rate: rate,
            };
            let id = db::product_create(&conn, &product)?;
            let mut out = product.clone();
            out.id = id;
            print_success(ctx, &out, |p| println!("added product '{}' (id {})", p.slug, p.id));
            Ok(())
        }
        ProductCmd::List => {
            let list = db::product_list(&conn)?;
            print_success(ctx, &list, |list| {
                if list.is_empty() {
                    println!("no products. add one: invoice products add <slug> --description ... --price 220.00 --currency SGD");
                }
                for p in list {
                    println!(
                        "{:<20}  {:<38}  {} / {}  {}%",
                        p.slug,
                        p.description,
                        p.unit_price.format_with_symbol(""),
                        p.unit,
                        p.tax_rate
                    );
                }
            });
            Ok(())
        }
        ProductCmd::Show { slug } => {
            let p = db::product_by_slug(&conn, &slug)?;
            print_success(ctx, &p, |p| println!("{:#?}", p));
            Ok(())
        }
        ProductCmd::Delete { slug } => {
            db::product_delete(&conn, &slug)?;
            print_success(ctx, &slug, |s| println!("deleted product '{s}'"));
            Ok(())
        }
        ProductCmd::Edit {
            slug,
            description,
            subtitle,
            unit,
            price,
            currency,
            tax_rate,
        } => {
            let mut product = db::product_by_slug(&conn, &slug)?;
            if let Some(v) = description {
                product.description = v;
            }
            if let Some(v) = subtitle {
                product.subtitle = Some(v);
            }
            if let Some(v) = unit {
                product.unit = v;
            }
            if let Some(v) = price {
                let price_dec = Decimal::from_str(&v)
                    .map_err(|e| AppError::InvalidInput(format!("bad price: {e}")))?;
                product.unit_price = MinorUnits::from_decimal(price_dec);
            }
            if let Some(v) = currency {
                product.currency = v;
            }
            if let Some(v) = tax_rate {
                let rate = Decimal::from_str(&v)
                    .map_err(|e| AppError::InvalidInput(format!("bad tax rate: {e}")))?;
                product.tax_rate = rate;
            }
            db::product_update(&conn, &product)?;
            print_success(ctx, &product, |p| {
                println!("updated product '{}' (id {})", p.slug, p.id)
            });
            Ok(())
        }
    }
}
