use anyhow::{Result, anyhow};
use clap::Args;
use std::thread;

use super::common::{
    Tokens, WorkflowConfig, describe_exit, list_macos_sound_names, play_notification_chime_with,
    prepare_blueprints, run_codex,
};
use crate::logging::log_blueprints;

const BUILDER_PROMPT_TEMPLATE: &str = include_str!("../prompts/delivery/BUILDER.md");
const REVIEWER_PROMPT_TEMPLATE: &str = include_str!("../prompts/delivery/REVIEWER.md");

#[derive(Args, Debug, Clone)]
pub struct DeliveryArgs {
    /// Workspace crate package name.
    #[arg(long = "crate", value_name = "crate", conflicts_with = "module_path")]
    pub crate_name: Option<String>,

    /// Optional module path within the workspace (e.g. `crates/crate_b/module_a`).
    #[arg(long = "module", value_name = "module-path")]
    pub module_path: Option<String>,

    /// macOS system sound name to play on success
    #[arg(long)]
    pub sound: Option<String>,

    /// List available macOS system sounds and exit
    #[arg(long)]
    pub list_sounds: bool,
}

pub fn handle(args: &DeliveryArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();

    let tokens = Tokens::new();
    let config = WorkflowConfig::from_env()?;

    let blueprints = prepare_blueprints(args.crate_name.as_deref(), args.module_path.as_deref())?;
    let reviewer_prompt = blueprints.apply(tokens.apply(REVIEWER_PROMPT_TEMPLATE));
    let builder_template = blueprints.apply(tokens.apply(BUILDER_PROMPT_TEMPLATE));

    let mut review_cycle = 0usize;

    loop {
        if review_cycle >= config.max_reviewer_iters {
            return Err(anyhow!(
                "review cycles exceeded MAX_REVIEWER_ITERS={}",
                config.max_reviewer_iters
            ));
        }
        review_cycle += 1;

        let reviewer = run_codex(
            &[
                "exec",
                "--model",
                "gpt-5",
                "--config",
                "model_reasoning_effort='high'",
                "--sandbox",
                "read-only",
                "--full-auto",
            ],
            &reviewer_prompt,
        )?;

        if !reviewer.status.success() {
            return Err(anyhow!(
                "reviewer codex exec failed (exit {})",
                describe_exit(reviewer.status)
            ));
        }

        let reviewer_trimmed = reviewer.stdout.trim();

        if reviewer_trimmed == tokens.error {
            return Err(anyhow!("reviewer reported {}", tokens.error));
        }

        // Reviewer sign-off only when entire output is exactly the COMPLETED token.
        if reviewer_trimmed == tokens.completed {
            log_blueprints("Reviewer sign-off detected");
            play_notification_chime_with(sound);
            return Ok(());
        }

        // Otherwise expect a CONTINUE block with reviewer feedback following the token line.
        let clean_feedback =
            extract_continue_payload(&reviewer.stdout, &tokens).ok_or_else(|| {
                anyhow!(
                    "reviewer must emit {} with actionable feedback",
                    tokens.continue_token
                )
            })?;

        if clean_feedback.is_empty() {
            return Err(anyhow!(
                "reviewer emitted no actionable feedback between control tokens"
            ));
        }

        run_builder_workflow(&builder_template, &tokens, &clean_feedback, &config)?;

        thread::sleep(config.loop_sleep);
    }
}

fn run_builder_workflow(
    builder_template: &str,
    tokens: &Tokens,
    clean_feedback: &str,
    config: &WorkflowConfig,
) -> Result<()> {
    let mut builder_iter = 0usize;
    loop {
        if builder_iter >= config.max_builder_iters {
            return Err(anyhow!(
                "builder loop exceeded MAX_BUILDER_ITERS={}",
                config.max_builder_iters
            ));
        }
        builder_iter += 1;

        let builder_prompt = builder_template.replace("${REVIEWER_FEEDBACK}", clean_feedback);
        let builder = run_codex(
            &[
                "exec",
                "--model",
                "gpt-5-codex",
                "--config",
                "model_reasoning_effort='high'",
                "--full-auto",
            ],
            &builder_prompt,
        )?;

        if !builder.status.success() {
            return Err(anyhow!(
                "builder codex exec failed (exit {})",
                describe_exit(builder.status)
            ));
        }

        let builder_last = builder.last_stdout_line.trim();

        if builder_last == tokens.error {
            return Err(anyhow!("builder reported {}", tokens.error));
        }

        if builder_last == tokens.completed {
            return Ok(());
        }

        if extract_continue_payload(&builder.stdout, tokens).is_some() {
            thread::sleep(config.loop_sleep);
            continue;
        }

        return Ok(());
    }
}

fn extract_continue_payload(output: &str, tokens: &Tokens) -> Option<String> {
    let mut found = false;
    let mut payload = Vec::new();

    for raw in output.lines() {
        let line = raw.trim_end_matches('\r');
        if !found {
            if line.trim() == tokens.continue_token {
                found = true;
                continue;
            }
            continue;
        }
        payload.push(line.to_string());
    }

    if found {
        Some(payload.join("\n").trim().to_string())
    } else {
        None
    }
}
