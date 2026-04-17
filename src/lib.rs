// Library entry — exposes render helpers so examples can reuse encode_qr.
// `money` and `tax` are re-exported from finance-core so the accounting suite
// shares one source of truth for currency/tax math. Internal callers keep
// using `crate::money::…` / `crate::tax::…` unchanged.
pub use finance_core::money;
pub use finance_core::tax;

pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod output;
pub mod render;
pub mod typst_assets;
