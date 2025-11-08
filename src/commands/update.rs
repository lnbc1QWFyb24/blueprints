use anyhow::{Context, Result};
use clap::Args;
use std::process::{Command, Stdio};

use super::common::{
    list_macos_sound_names, play_notification_chime_with, prepare_blueprints_for_crate,
    prepare_blueprints_for_module, resolve_target_from_crate, resolve_target_from_module_path,
};
use crate::prompts::builder::Profile;

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    /// Target Cargo package name (crate)
    #[arg(
        long = "crate",
        value_name = "PKG",
        conflicts_with = "module",
        required_unless_present = "module"
    )]
    pub krate: Option<String>,

    /// Target module path (file or dir) inside a crate
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with = "krate",
        required_unless_present = "krate"
    )]
    pub module: Option<String>,

    /// macOS system sound name to play on success
    #[arg(long)]
    pub sound: Option<String>,

    /// List available macOS system sounds and exit
    #[arg(long)]
    pub list_sounds: bool,
}

pub fn handle(args: &UpdateArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();

    // Resolve target strictly from flags and anchor at the scoped blueprints directory
    let target = if let Some(name) = &args.krate {
        resolve_target_from_crate(name)?
    } else if let Some(path) = &args.module {
        resolve_target_from_module_path(path)?
    } else {
        return Err(anyhow::anyhow!(
            "specify exactly one of --crate or --module"
        ));
    };
    let blueprints = if target.module_rel.is_some() {
        prepare_blueprints_for_module(&target)?
    } else {
        prepare_blueprints_for_crate(&target)?
    };
    let prompt = Profile::Update
        .compose()
        .with_blueprints_dir(blueprints.dir_token_value())
        .inline_blueprints()
        .build()?;

    // Interactive run (allow writes to apply updates)
    let status = Command::new("codex")
        .args(["--profile", "reviewer"])
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
