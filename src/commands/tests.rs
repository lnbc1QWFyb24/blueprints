use anyhow::{Result, anyhow};
use clap::Args;
use std::thread;

use super::common::{
    Tokens, WorkflowConfig, describe_exit, list_macos_sound_names, play_notification_chime_with,
    run_codex,
};
use crate::logging::log_blueprints;

const BUILDER_PROMPT_TEMPLATE: &str = include_str!("../prompts/tests/BUILDER.md");
const REVIEWER_PROMPT_TEMPLATE: &str = include_str!("../prompts/tests/REVIEWER.md");

#[derive(Args, Debug, Clone)]
pub struct TestsArgs {
    /// macOS system sound name to play on success
    #[arg(long)]
    pub sound: Option<String>,

    /// List available macOS system sounds and exit
    #[arg(long)]
    pub list_sounds: bool,
}

pub fn handle(args: &TestsArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();

    let tokens = Tokens::new();
    let config = WorkflowConfig::from_env()?;

    let reviewer_prompt = tokens.apply(REVIEWER_PROMPT_TEMPLATE);
    let builder_template = tokens.apply(BUILDER_PROMPT_TEMPLATE);

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

        if reviewer_trimmed == tokens.completed {
            log_blueprints("Reviewer sign-off detected");
            play_notification_chime_with(sound);
            return Ok(());
        }

        let plan = extract_plan(&reviewer.stdout)
            .ok_or_else(|| anyhow!("reviewer did not emit a parseable Implementation Plan"))?;

        if plan.trim().is_empty() {
            return Err(anyhow!(
                "reviewer emitted empty plan between ---PLAN START--- and ---PLAN END---"
            ));
        }

        let mut builder_iter = 0usize;
        let mut builder_completed = false;

        while builder_iter < config.max_builder_iters {
            builder_iter += 1;

            let builder_prompt = builder_template.replace("${IMPLEMENTATION_PLAN}", &plan);
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

            let builder_trimmed = builder.stdout.trim();

            if builder_trimmed == tokens.error {
                return Err(anyhow!("builder reported {}", tokens.error));
            }

            if builder_trimmed == tokens.completed {
                builder_completed = true;
                break;
            }

            if builder_trimmed == tokens.continue_token {
                thread::sleep(config.loop_sleep);
                continue;
            }

            // No control token means single-pass apply succeeded.
            builder_completed = true;
            break;
        }

        if !builder_completed {
            return Err(anyhow!(
                "builder loop exceeded MAX_BUILDER_ITERS={}",
                config.max_builder_iters
            ));
        }

        thread::sleep(config.loop_sleep);
    }
}

fn extract_plan(output: &str) -> Option<String> {
    let mut in_plan = false;
    let mut lines = Vec::new();

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line == "---PLAN START---" {
            in_plan = true;
            continue;
        }
        if line == "---PLAN END---" {
            if in_plan {
                return Some(lines.join("\n"));
            }
            break;
        }
        if in_plan {
            lines.push(line.to_string());
        }
    }

    None
}
