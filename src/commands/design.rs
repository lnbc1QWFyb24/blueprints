use anyhow::{Context, Result};
use clap::Args;
use std::{
    fs,
    process::{Command, Stdio},
};

use super::common::{
    list_macos_sound_names, play_notification_chime_with, prepare_blueprints_for_module,
    resolve_target_from_crate, resolve_target_from_module_path,
};
use crate::prompts::builder::Profile;

#[derive(Args, Debug, Clone)]
pub struct DesignArgs {
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

pub fn handle(args: &DesignArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();

    // Resolve target strictly from flags
    let target = if let Some(name) = &args.krate {
        resolve_target_from_crate(name)?
    } else if let Some(path) = &args.module {
        resolve_target_from_module_path(path)?
    } else {
        return Err(anyhow::anyhow!(
            "specify exactly one of --crate or --module"
        ));
    };

    // Ensure blueprints directory exists at the module root when --module is given; else crate root.
    let crate_root_abs = if target.crate_root.is_absolute() {
        target.crate_root.clone()
    } else {
        target.workspace_root.join(&target.crate_root)
    };
    let module_root_abs = if let Some(rel) = &target.module_rel {
        let abs = crate_root_abs.join(rel);
        if abs.is_dir() { abs } else { abs.parent().map(|p| p.to_path_buf()).unwrap_or(crate_root_abs) }
    } else {
        crate_root_abs
    };
    let target_dir = module_root_abs.join("blueprints");
    fs::create_dir_all(&target_dir).with_context(|| {
        format!(
            "failed to create blueprints directory at {}",
            target_dir.display()
        )
    })?;

    let blueprints = prepare_blueprints_for_module(&target)?;
    let blueprint_dir_token = blueprints.dir_token_value();
    let prompt = Profile::Design
        .compose()
        .with_blueprints_dir(blueprint_dir_token)
        .inline_blueprints()
        .build()?;

    // Run codex interactively to allow the human to answer questions; allow writes (no read-only sandbox)
    let status = Command::new("codex")
        .args(["--profile", "reviewer"]) // reviewer profile for interactive, Q&A driven flow
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
