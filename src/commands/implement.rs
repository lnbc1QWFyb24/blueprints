use anyhow::{Context, Result, anyhow};
use clap::Args;
use std::{
    fmt::Write as _,
    fs,
    path::Path,
    process::{Command, Stdio},
    thread,
};

use super::common::{
    Tokens, WorkflowConfig, describe_exit, list_macos_sound_names, play_notification_chime_with,
    prepare_blueprints_for_crate, run_codex,
};
use crate::logging::log_blueprints;
use crate::prompts::builder::Profile;

#[derive(Args, Debug)]
pub struct ImplementArgs {
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

#[allow(clippy::too_many_lines)]
pub fn handle(args: &ImplementArgs) -> Result<()> {
    if args.list_sounds {
        for name in list_macos_sound_names() {
            println!("{name}");
        }
        return Ok(());
    }
    let sound = args.sound.as_deref();
    let tokens = Tokens::new();
    let config = WorkflowConfig::from_env()?;

    // Resolve target strictly from flags
    let target = if let Some(name) = &args.krate {
        super::common::resolve_target_from_crate(name)
    } else if let Some(path) = &args.module {
        super::common::resolve_target_from_module_path(path)
    } else {
        // Clap should enforce one of them; keep a defensive error.
        return Err(anyhow!("specify exactly one of --crate or --module"));
    }?;
    // Prefer module-level blueprints when a module path is provided; otherwise fall back to crate-level
    let blueprints = if target.module_rel.is_some() {
        super::common::prepare_blueprints_for_module(&target)?
    } else {
        prepare_blueprints_for_crate(&target)?
    };
    let module = target.crate_name.as_str();
    let delivery_plan_path = blueprints.join("05-delivery-plan.md");
    let has_cargo_toml = Path::new("Cargo.toml").exists();

    let mut ci_state = CiState::default();

    let blueprint_dir_token = blueprints.dir_token_value();
    // Compose additional scope tokens for prompts
    let crate_root_token = {
        let p = &target.crate_root;
        let s = p.to_string_lossy();
        if p.is_relative() && !s.starts_with("./") && !s.starts_with("../") {
            format!("./{s}")
        } else {
            s.into_owned()
        }
    };
    let module_rel_token = target.module_rel.as_ref().map(|p| {
        let s = p.to_string_lossy();
        if p.is_relative() && !s.starts_with("./") && !s.starts_with("../") {
            format!("./{s}")
        } else {
            s.into_owned()
        }
    });

    // Compose reviewer prompt (runtime) from modular sections + reviewer specifics
    let mut reviewer_builder = Profile::ImplementReviewer
        .compose()
        .with_blueprints_dir(blueprint_dir_token.clone())
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token.clone())
        .inline_blueprints();
    if let Some(mrel) = &module_rel_token {
        reviewer_builder = reviewer_builder.with_variable("MODULE_REL_PATH", mrel.clone());
    }
    let reviewer_template = tokens.apply(&reviewer_builder.build()?);

    // Compose builder prompt (runtime) from modular sections + builder specifics
    let mut builder_builder = Profile::ImplementBuilder
        .compose()
        .with_blueprints_dir(blueprint_dir_token)
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token);
    if let Some(mrel) = module_rel_token {
        builder_builder = builder_builder.with_variable("MODULE_REL_PATH", mrel);
    }
    builder_builder = builder_builder.inline_blueprints();
    let builder_template = tokens.apply(&builder_builder.build()?);

    let mut review_cycle = 0usize;
    loop {
        if review_cycle >= config.max_reviewer_iters {
            return Err(anyhow!(
                "review cycles exceeded MAX_REVIEWER_ITERS={}",
                config.max_reviewer_iters
            ));
        }
        review_cycle += 1;

        let host_ci_results = compute_host_ci_results(&ci_state, has_cargo_toml);
        let reviewer_prompt = reviewer_template.replace("${HOST_CI_RESULTS}", &host_ci_results);

        log_blueprints("RUNNING REVIEWER AGENT");
        let reviewer = run_codex(
            &[
                "exec",
                "--model",
                "gpt-5",
                "--config",
                "model_reasoning_effort='high'",
                "--config",
                "web_search_request=true",
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

        let mut reviewer_output = reviewer.stdout.clone();
        let reviewer_trimmed = reviewer.stdout.trim();

        if reviewer_trimmed == tokens.error {
            return Err(anyhow!("reviewer reported {}", tokens.error));
        }

        // Reviewer sign-off only when entire output is exactly the COMPLETED token.
        if reviewer_trimmed == tokens.completed {
            // If any Delivery Plan items remain unchecked, force a CONTINUE with the list.
            let unchecked_items = enumerate_unchecked_items(&delivery_plan_path)?;
            if !unchecked_items.is_empty() {
                let formatted = format_enumerated(&unchecked_items);
                println!("{}", tokens.continue_token);
                if !formatted.is_empty() {
                    println!("{formatted}");
                }
                reviewer_output = format!("{}\n{}", tokens.continue_token, formatted);
            } else if !has_cargo_toml {
                log_blueprints("Reviewer sign-off detected");
                play_notification_chime_with(sound);
                return Ok(());
            } else {
                ci_state.failure_output.clear();

                match run_ci_checks(module)? {
                    CiOutcome::Success { summary } => {
                        ci_state.mode = CiMode::Known;
                        ci_state.last_summary = summary;
                        log_blueprints(
                            "Reviewer sign-off detected; cargo fmt/clippy/check/nextest all passed",
                        );
                        play_notification_chime_with(sound);
                        return Ok(());
                    }
                    CiOutcome::Failures { summary, feedback } => {
                        run_ci_fixer_loop(module, &config, &mut ci_state, summary, feedback)?;
                        log_blueprints("CI errors resolved; rerunning reviewer for final sign-off");
                        continue;
                    }
                    CiOutcome::CargoMissing { summary, feedback } => {
                        ci_state.mode = CiMode::Known;
                        ci_state.last_summary = summary;
                        ci_state.failure_output.clone_from(&feedback);
                        println!("{}", tokens.continue_token);
                        if !feedback.is_empty() {
                            println!("{feedback}");
                        }
                        reviewer_output = format!("{}\n{}", tokens.continue_token, feedback);
                    }
                }
            }
        }

        // Otherwise expect a CONTINUE block with remaining work lines following the token.
        let mut remaining_work =
            extract_continue_payload(&reviewer_output, &tokens).ok_or_else(|| {
                anyhow!(
                    "reviewer must emit ${} with remaining work list; got no control token",
                    tokens.continue_token
                )
            })?;
        if remaining_work.is_empty() {
            return Err(anyhow!(
                "reviewer emitted no actionable feedback between control tokens"
            ));
        }

        let mut builder_iter = 0usize;
        let mut builder_completed = false;

        while builder_iter < config.max_builder_iters {
            builder_iter += 1;

            let builder_prompt =
                builder_template.replace("${REVIEWER_FEEDBACK_OR_REMAINING_WORK}", &remaining_work);

            log_blueprints("RUNNING BUILDER AGENT");
            let builder = run_codex(
                &[
                    "exec",
                    "--model",
                    // "gpt-5-codex",
                    "gpt-5-codex",
                    "--config",
                    "model_reasoning_effort='high'",
                    "--config",
                    "web_search_request=true",
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

            // Completed only if the final line equals the COMPLETED token.
            if builder_last == tokens.completed {
                let unchecked_items = enumerate_unchecked_items(&delivery_plan_path)?;
                if !unchecked_items.is_empty() {
                    let formatted = format_enumerated(&unchecked_items);
                    println!("{}", tokens.continue_token);
                    if !formatted.is_empty() {
                        println!("{formatted}");
                    }
                    remaining_work = formatted;
                    thread::sleep(config.loop_sleep);
                    continue;
                }

                builder_completed = true;
                break;
            }

            // If a CONTINUE token exists, extract only the payload after the first token line.
            if let Some(next_work) = extract_continue_payload(&builder.stdout, &tokens) {
                if !next_work.is_empty() {
                    remaining_work = next_work;
                }
                thread::sleep(config.loop_sleep);
                continue;
            }

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

fn extract_continue_payload(output: &str, tokens: &Tokens) -> Option<String> {
    let mut found = false;
    let mut payload = Vec::new();

    for raw in output.lines() {
        let line = raw.trim_end_matches('\r');
        if !found {
            if line.trim() == tokens.continue_token {
                found = true;
                // Everything after this line is payload
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

fn enumerate_unchecked_items(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let mut items = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('-') {
            continue;
        }

        let mut after_dash = trimmed[1..].trim_start();
        if !after_dash.starts_with('[') {
            continue;
        }

        if let Some(close_idx) = after_dash.find(']') {
            let inside = &after_dash[1..close_idx];
            if !inside.trim().is_empty() {
                continue;
            }

            after_dash = after_dash[close_idx + 1..].trim_start();
            if !after_dash.is_empty() {
                items.push(after_dash.to_string());
            }
        }
    }

    Ok(items)
}

fn format_enumerated(items: &[String]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| format!("{}) {}", idx + 1, item))
        .collect::<Vec<_>>()
        .join("\n")
}

fn compute_host_ci_results(ci_state: &CiState, has_cargo_toml: bool) -> String {
    if !has_cargo_toml {
        return "none (no Cargo.toml)".to_string();
    }

    match ci_state.mode {
        CiMode::Pending => {
            "cargo_fmt_check=pending\ncargo_clippy=pending\ncargo_check=pending\ncargo_nextest=pending"
                .to_string()
        }
        CiMode::Known => {
            let mut results = ci_state.last_summary.clone();
            if !ci_state.failure_output.trim().is_empty() {
                if results.is_empty() {
                    results.push_str("CI_FAILURE_OUTPUT\n");
                } else {
                    results.push_str("\n\nCI_FAILURE_OUTPUT\n");
                }
                results.push_str(ci_state.failure_output.trim_end());
            }
            results
        }
    }
}

fn run_ci_checks(module: &str) -> Result<CiOutcome> {
    if !cargo_available() {
        let summary = "cargo_fmt_check=blocked\ncargo_clippy=blocked\ncargo_check=blocked\ncargo_nextest=blocked".to_string();
        let feedback = "1) CI:cargo command not found on PATH. Install Rust toolchain so cargo fmt/clippy/check/nextest can run.".to_string();
        return Ok(CiOutcome::CargoMissing { summary, feedback });
    }

    let command_specs = vec![
        CiCommand {
            key: "cargo_fmt_check".to_string(),
            args: ["fmt", "--all", "--", "--check"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
        CiCommand {
            key: "cargo_clippy".to_string(),
            args: vec![
                "clippy".to_string(),
                "-p".to_string(),
                module.to_string(),
                "--all-targets".to_string(),
                "--all-features".to_string(),
                "--".to_string(),
                "-W".to_string(),
                "clippy::all".to_string(),
                "-W".to_string(),
                "clippy::pedantic".to_string(),
            ],
        },
        CiCommand {
            key: "cargo_check".to_string(),
            args: vec![
                "check".to_string(),
                "-p".to_string(),
                module.to_string(),
                "--all-targets".to_string(),
                "--all-features".to_string(),
            ],
        },
        CiCommand {
            key: "cargo_nextest".to_string(),
            args: vec![
                "nextest".to_string(),
                "run".to_string(),
                "-p".to_string(),
                module.to_string(),
                "--all-features".to_string(),
            ],
        },
    ];

    let mut summary_entries = Vec::new();
    let mut failures = Vec::new();

    for spec in command_specs {
        let subcommand = spec.args.first().map_or("<unknown>", String::as_str);

        let output = Command::new("cargo")
            .args(&spec.args)
            .output()
            .with_context(|| format!("failed to run cargo {subcommand}"))?;

        let status = if output.status.success() {
            "pass"
        } else {
            "fail"
        };
        summary_entries.push(format!("{}={}", spec.key, status));

        if !output.status.success() {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            failures.push(CiFailure {
                key: spec.key,
                exit: describe_exit(output.status),
                output: combined,
            });
        }
    }

    let summary = summary_entries.join("\n");

    if failures.is_empty() {
        return Ok(CiOutcome::Success { summary });
    }

    let mut feedback = String::new();
    for (idx, failure) in failures.iter().enumerate() {
        let index = idx + 1;
        let _ = write!(
            feedback,
            "{index}) CI:{} failed (exit {}).\n{}\n",
            failure.key, failure.exit, failure.output
        );
    }

    Ok(CiOutcome::Failures {
        summary,
        feedback: feedback.trim_end().to_string(),
    })
}

fn run_ci_fixer_loop(
    module: &str,
    config: &WorkflowConfig,
    ci_state: &mut CiState,
    initial_summary: String,
    initial_feedback: String,
) -> Result<()> {
    let mut attempt = 0usize;
    let mut summary = initial_summary;
    let mut feedback = initial_feedback;

    loop {
        attempt += 1;
        if attempt > config.max_builder_iters {
            return Err(anyhow!(
                "ci fixer loop exceeded MAX_BUILDER_ITERS={}",
                config.max_builder_iters
            ));
        }

        ci_state.mode = CiMode::Known;
        ci_state.last_summary.clone_from(&summary);
        ci_state.failure_output.clone_from(&feedback);

        let prompt = format!("Fix the following CI errors: {feedback}");
        log_blueprints("RUNNING CI FIXER AGENT");
        let fixer = run_codex(&["exec", "--profile", "builder", "--full-auto"], &prompt)?;

        if !fixer.status.success() {
            return Err(anyhow!(
                "ci fixer codex exec failed (exit {})",
                describe_exit(fixer.status)
            ));
        }

        thread::sleep(config.loop_sleep);

        match run_ci_checks(module)? {
            CiOutcome::Success {
                summary: success_summary,
            } => {
                ci_state.mode = CiMode::Known;
                ci_state.last_summary = success_summary;
                ci_state.failure_output.clear();
                return Ok(());
            }
            CiOutcome::Failures {
                summary: next_summary,
                feedback: next_feedback,
            } => {
                summary = next_summary;
                feedback = next_feedback;
            }
            CiOutcome::CargoMissing {
                summary: missing_summary,
                feedback: missing_feedback,
            } => {
                ci_state.mode = CiMode::Known;
                ci_state.last_summary = missing_summary;
                ci_state.failure_output = missing_feedback;
                return Err(anyhow!("CI blocked: cargo command not found on PATH."));
            }
        }
    }
}

fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

#[derive(Default)]
struct CiState {
    mode: CiMode,
    last_summary: String,
    failure_output: String,
}

#[derive(Default)]
enum CiMode {
    #[default]
    Pending,
    Known,
}

struct CiCommand {
    key: String,
    args: Vec<String>,
}

struct CiFailure {
    key: String,
    exit: String,
    output: String,
}

enum CiOutcome {
    Success { summary: String },
    Failures { summary: String, feedback: String },
    CargoMissing { summary: String, feedback: String },
}
