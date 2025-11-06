use anyhow::{Context, Result};
use clap::Args;
use std::process::{Command, Stdio};

use super::common::{
    WorkflowMode, list_macos_sound_names, play_notification_chime_with, prepare_blueprints,
};

const DESIGN_PROMPT: &str = include_str!("../prompts/requirements/DESIGN.md");
const UPDATE_PROMPT: &str = include_str!("../prompts/requirements/UPDATE.md");

#[derive(Args)]
pub struct RequirementsArgs {
    /// Workspace crate package name.
    #[arg(long = "crate", value_name = "crate", conflicts_with = "module_path")]
    pub crate_name: Option<String>,

    /// Optional module path within the workspace (e.g. `crates/crate_b/module_a`).
    #[arg(long = "module", value_name = "module-path")]
    pub module_path: Option<String>,

    /// Requirements workflow mode.
    #[arg(long, value_enum)]
    pub mode: WorkflowMode,

    /// macOS system sound name to play on success
    #[arg(long)]
    pub sound: Option<String>,

    /// List available macOS system sounds and exit
    #[arg(long)]
    pub list_sounds: bool,
}

pub fn handle(args: &RequirementsArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();

    let blueprints = prepare_blueprints(args.crate_name.as_deref(), args.module_path.as_deref())?;
    let template = match args.mode {
        WorkflowMode::Design => DESIGN_PROMPT,
        WorkflowMode::Update => UPDATE_PROMPT,
    };
    let prompt = blueprints.apply(template);

    let status = Command::new("codex")
        .args([
            "--model",
            "gpt-5",
            "--config",
            "model_reasoning_effort='high'",
            "--full-auto",
        ])
        .arg(prompt)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to execute codex CLI")?;

    if status.success() {
        play_notification_chime_with(sound);
    }

    std::process::exit(status.code().unwrap_or(1));
}
