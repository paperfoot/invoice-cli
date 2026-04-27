use crate::cli::{Cli, Commands};
use crate::error::Result;
use crate::output::Ctx;

pub mod agent_info;
pub mod clients;
pub mod config;
pub mod doctor;
pub mod invoices;
pub mod issuers;
pub mod products;
pub mod skill;
pub mod template;
pub mod update;

pub(crate) fn split_multiline_arg(value: &str) -> Vec<String> {
    let normalized = value.replace("\\n", "\n");
    normalized.split('\n').map(|s| s.to_string()).collect()
}

pub fn dispatch(cli: Cli, ctx: Ctx) -> Result<()> {
    crate::config::ensure_dirs()?;
    crate::typst_assets::ensure_extracted()?;

    match cli.command {
        Commands::Issuers(cmd) => issuers::run(cmd, ctx),
        Commands::Clients(cmd) => clients::run(cmd, ctx),
        Commands::Products(cmd) => products::run(cmd, ctx),
        Commands::Invoices(cmd) => invoices::run(cmd, ctx),
        Commands::Template(cmd) => template::run(cmd, ctx),
        Commands::Config(cmd) => config::run(cmd, ctx),
        Commands::AgentInfo => agent_info::run(ctx),
        Commands::Skill(cmd) => skill::run(cmd, ctx),
        Commands::Doctor => doctor::run(ctx),
        Commands::Update { check } => update::run(ctx, check),
    }
}
