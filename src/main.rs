mod commands;
mod logging;
mod prompts;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::prompts::{self as prompts_cmd, PromptsArgs};
use commands::{
    design::{self, DesignArgs},
    implement::{self, ImplementArgs},
    review::{self, ReviewArgs},
    update::{self, UpdateArgs},
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
    /// Design and create the complete blueprints set interactively.
    Design(DesignArgs),
    /// Review the entire blueprints set for quality and completeness.
    Review(ReviewArgs),
    /// Update the blueprints set interactively (adds/removes/edits + lifecycle).
    Update(UpdateArgs),
    /// Render the composed prompts for each workflow.
    Prompts(PromptsArgs),
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
        Commands::Design(args) => design::handle(&args)?,
        Commands::Review(args) => review::handle(&args)?,
        Commands::Update(args) => update::handle(&args)?,
        Commands::Prompts(args) => prompts_cmd::handle(&args)?,
        Commands::Implement(args) => implement::handle(&args)?,
    }

    Ok(())
}
