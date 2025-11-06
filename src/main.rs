mod commands;
mod logging;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{
    contracts::{self, ContractsArgs},
    delivery::{self, DeliveryArgs},
    implement::{self, ImplementArgs},
    requirements::{self, RequirementsArgs},
    specs::{self, SpecsArgs},
    tests::{self, TestsArgs},
};
use logging::log_error;

#[derive(Parser)]
#[command(
    name = "blueprints",
    version,
    about = "Blueprints CLI tooling",
    long_about = "TODO"
)]
struct Cli {
    /// Enable live Codex output summarization
    #[arg(long, global = true)]
    summarize: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Workflow for managing `blueprints/01-requirements.md` WHAT/WHY records (coming soon).
    Requirements(RequirementsArgs),
    /// Workflow for maintaining `blueprints/02-spec.md` HOW clauses (coming soon).
    Specs(SpecsArgs),
    /// Workflow for defining `blueprints/03-contracts.md` types and APIs (coming soon).
    Contracts(ContractsArgs),
    /// Workflow for curating `blueprints/04-test-vectors.md` canonical cases (coming soon).
    Tests(TestsArgs),
    /// Workflow for shaping the `blueprints/05-delivery-plan.md` checklist (coming soon).
    Delivery(DeliveryArgs),
    /// Workflow that guides translating approved blueprints into code (coming soon).
    Implement(ImplementArgs),
}

fn main() {
    logging::init();

    if let Err(error) = run() {
        log_error(format!("application error: {error}"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Configure global summarization mode (opt-in; default disabled)
    commands::common::set_summarize_enabled(cli.summarize);

    match cli.command {
        Commands::Requirements(args) => requirements::handle(&args)?,
        Commands::Specs(args) => specs::handle(&args)?,
        Commands::Contracts(args) => contracts::handle(&args)?,
        Commands::Tests(args) => tests::handle(&args)?,
        Commands::Delivery(args) => delivery::handle(&args)?,
        Commands::Implement(args) => implement::handle(&args)?,
    }

    Ok(())
}
