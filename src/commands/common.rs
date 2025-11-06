use crate::logging::log_codex;
use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use std::{
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

pub(crate) const COMPLETED_TOKEN: &str = "__BLUEPRINTS_COMPLETED__";
pub(crate) const CONTINUE_TOKEN: &str = "__BLUEPRINTS_CONTINUE__";
pub(crate) const ERROR_TOKEN: &str = "__BLUEPRINTS_ERROR__";

static SUMMARIZE_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum WorkflowMode {
    Design,
    Update,
}

pub(crate) fn set_summarize_enabled(enabled: bool) {
    let _ = SUMMARIZE_ENABLED.set(enabled);
}

fn summarize_enabled() -> bool {
    *SUMMARIZE_ENABLED.get_or_init(|| false)
}

pub(crate) struct WorkflowConfig {
    pub(crate) max_builder_iters: usize,
    pub(crate) max_reviewer_iters: usize,
    pub(crate) loop_sleep: Duration,
}

impl WorkflowConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let max_builder_iters = parse_env_usize("MAX_BUILDER_ITERS", 50)?;
        let max_reviewer_iters = parse_env_usize("MAX_REVIEWER_ITERS", 100)?;
        let loop_sleep_secs = parse_env_f64("LOOP_SLEEP_SECS", 0.2)?;

        if loop_sleep_secs < 0.0 {
            return Err(anyhow!("LOOP_SLEEP_SECS must be non-negative"));
        }

        Ok(Self {
            max_builder_iters,
            max_reviewer_iters,
            loop_sleep: Duration::from_secs_f64(loop_sleep_secs),
        })
    }
}

pub(crate) struct Tokens {
    pub(crate) completed: &'static str,
    pub(crate) continue_token: &'static str,
    pub(crate) error: &'static str,
}

impl Tokens {
    pub(crate) fn new() -> Self {
        Self {
            completed: COMPLETED_TOKEN,
            continue_token: CONTINUE_TOKEN,
            error: ERROR_TOKEN,
        }
    }

    pub(crate) fn apply(&self, template: &str) -> String {
        template
            .replace("${COMPLETED_TOKEN}", self.completed)
            .replace("${CONTINUE_TOKEN}", self.continue_token)
            .replace("${ERROR_TOKEN}", self.error)
    }
}

pub(crate) struct BlueprintsContext {
    package: String,
    blueprints_dir: PathBuf,
}

impl BlueprintsContext {
    pub(crate) fn module(&self) -> &str {
        &self.package
    }

    pub(crate) fn join(&self, file: &str) -> PathBuf {
        self.blueprints_dir.join(file)
    }

    pub(crate) fn apply(&self, template: impl AsRef<str>) -> String {
        let dir_cow = self.blueprints_dir.to_string_lossy();
        let dir = if self.blueprints_dir.is_relative()
            && !dir_cow.starts_with("./")
            && !dir_cow.starts_with("../")
        {
            format!("./{dir_cow}")
        } else {
            dir_cow.into_owned()
        };

        template.as_ref().replace("${BLUEPRINTS_DIR}", &dir)
    }
}

pub(crate) fn prepare_blueprints(
    crate_name: Option<&str>,
    module_path: Option<&str>,
) -> Result<BlueprintsContext> {
    let original_cwd =
        env::current_dir().context("failed to determine current working directory")?;
    let workspace_root = {
        let root = find_workspace_root()?;
        root.canonicalize().unwrap_or(root)
    };
    env::set_current_dir(&workspace_root).context("failed to switch to workspace root")?;

    let module_dir = module_path
        .map(|module| resolve_module_dir(&workspace_root, module))
        .transpose()?;

    let explicit_crate_dir = crate_name
        .map(|name| resolve_crate_dir(&workspace_root, name))
        .transpose()?;

    let crate_root = if let Some(dir) = explicit_crate_dir.clone() {
        Some(dir)
    } else if let Some(module_dir) = &module_dir {
        find_crate_root(module_dir, &workspace_root)
    } else {
        find_crate_root(&original_cwd, &workspace_root)
    };

    let target_path = module_dir
        .clone()
        .or(explicit_crate_dir.clone())
        .unwrap_or_else(|| original_cwd.clone());

    let mut search_roots = Vec::new();

    if let Some(dir) = module_dir.clone() {
        search_roots.push(dir);
    }

    if let Some(root) = crate_root.as_ref()
        && search_roots.iter().all(|p| p != root)
    {
        search_roots.push(root.clone());
    }

    if search_roots.is_empty() {
        search_roots.push(target_path.clone());
    }

    let (blueprints_dir, root_used) = locate_or_create_blueprints(&search_roots)?;
    let package = infer_package_name(crate_name, crate_root.as_ref(), root_used.as_path());

    Ok(BlueprintsContext {
        package,
        blueprints_dir,
    })
}

fn resolve_module_dir(workspace_root: &Path, module_path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(module_path);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        workspace_root.join(&candidate)
    };

    if !joined.exists() {
        return Err(anyhow!("module path '{}' does not exist", joined.display()));
    }

    if !joined.is_dir() {
        return Err(anyhow!(
            "module path '{}' is not a directory",
            joined.display()
        ));
    }

    let resolved = joined
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", joined.display()))?;

    if !resolved.starts_with(workspace_root) {
        return Err(anyhow!(
            "module path '{}' escapes the workspace root {}",
            resolved.display(),
            workspace_root.display()
        ));
    }

    Ok(resolved)
}

fn resolve_crate_dir(workspace_root: &Path, crate_name: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(crate_name);
    let mut possibilities = Vec::new();

    if candidate.is_absolute() {
        possibilities.push(candidate);
    } else {
        possibilities.push(workspace_root.join(&candidate));
        possibilities.push(workspace_root.join("crates").join(&candidate));
    }

    for path in possibilities {
        if path.exists() && path.is_dir() {
            let resolved = path
                .canonicalize()
                .with_context(|| format!("failed to canonicalize {}", path.display()))?;

            if !resolved.starts_with(workspace_root) {
                return Err(anyhow!(
                    "crate '{}' resolved to '{}' which escapes workspace root {}",
                    crate_name,
                    resolved.display(),
                    workspace_root.display()
                ));
            }
            return Ok(resolved);
        }
    }

    Err(anyhow!(
        "could not resolve crate '{}' relative to {}",
        crate_name,
        workspace_root.display()
    ))
}

fn locate_or_create_blueprints(roots: &[PathBuf]) -> Result<(PathBuf, PathBuf)> {
    assert!(!roots.is_empty(), "expected at least one search root");

    for root in roots {
        let candidate = root.join("blueprints");
        if candidate.exists() {
            return Ok((candidate, root.clone()));
        }
    }

    let Some(primary) = roots.first() else {
        unreachable!("expected at least one search root");
    };
    let blueprints_dir = primary.join("blueprints");
    fs::create_dir_all(&blueprints_dir).with_context(|| {
        format!(
            "failed to create blueprints directory at {}",
            blueprints_dir.display()
        )
    })?;

    Ok((blueprints_dir, primary.clone()))
}

fn infer_package_name(
    explicit_crate: Option<&str>,
    crate_root: Option<&PathBuf>,
    root_used: &Path,
) -> String {
    if let Some(name) = explicit_crate {
        return name.to_string();
    }

    if let Some(root) = crate_root {
        if let Some(name) = read_package_name(root) {
            return name;
        }

        if let Some(name) = root.file_name().and_then(|s| s.to_str()) {
            return name.to_string();
        }
    }

    root_used
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("blueprints")
        .to_string()
}

fn read_package_name(crate_root: &Path) -> Option<String> {
    let manifest_path = crate_root.join("Cargo.toml");
    let content = fs::read_to_string(manifest_path).ok()?;

    let mut in_package = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }

        if !in_package || !line.starts_with("name") {
            continue;
        }

        if let Some((_, value)) = line.split_once('=') {
            let trimmed = value.trim().trim_matches('"');
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn find_crate_root(start: &Path, workspace_root: &Path) -> Option<PathBuf> {
    let mut current = if start.is_dir() {
        start
    } else {
        start.parent()?
    };

    loop {
        if current.join("Cargo.toml").is_file() {
            return Some(current.to_path_buf());
        }

        if current == workspace_root {
            break;
        }

        current = match current.parent() {
            Some(parent) => parent,
            None => break,
        };
    }

    if workspace_root.join("Cargo.toml").is_file() {
        Some(workspace_root.to_path_buf())
    } else {
        None
    }
}

pub(crate) fn find_workspace_root() -> Result<PathBuf> {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(PathBuf::from(path))
        }
        _ => env::current_dir().context("failed to determine current working directory"),
    }
}

// macOS-only implementation: use afplay for named system sounds, fallback to osascript beep
#[cfg(target_os = "macos")]
pub(crate) fn play_notification_chime_with(name: Option<&str>) {
    // Emit terminal BEL first
    let _ = io::stdout().write_all(b"\x07");
    let _ = io::stdout().flush();

    if env::var("BLUEPRINTS_NO_CHIME_FALLBACKS").is_ok() {
        return;
    }

    let env_choice = env::var("BLUEPRINTS_CHIME").ok();
    let selected = name.map(str::to_string).or(env_choice);

    // If a specific name is given, resolve via directory scan first (supports all installed sounds)
    if let Some(sel) = selected.as_deref()
        && let Some(path) = resolve_macos_sound_path(sel)
        && run_quiet("afplay", &[&path])
    {
        return;
    }

    // Fallback to a default if none chosen or afplay failed
    if run_quiet("afplay", &["/System/Library/Sounds/Ping.aiff"]) {
        return;
    }
    let _ = run_quiet("osascript", &["-e", "beep"]);
}

// Stub for non-macOS builds to keep compilation possible; does nothing beyond BEL
#[cfg(not(target_os = "macos"))]
pub(crate) fn play_notification_chime_with(_name: Option<&str>) {
    let _ = io::stdout().write_all(b"\x07");
    let _ = io::stdout().flush();
}

// macOS: list available system sounds (names only)
#[cfg(target_os = "macos")]
pub(crate) fn list_macos_sound_names() -> Vec<String> {
    let entries = collect_macos_sounds();
    let mut names: Vec<String> = entries.into_iter().map(|(name, _)| name).collect();
    names.sort_by_key(|a| a.to_lowercase());
    names
}

// macOS: resolve a name to an absolute file path
#[cfg(target_os = "macos")]
pub(crate) fn resolve_macos_sound_path(name: &str) -> Option<String> {
    let name_lc = name.to_ascii_lowercase();
    let entries = collect_macos_sounds();
    for (display, path) in entries {
        if display.to_ascii_lowercase() == name_lc {
            return Some(path);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn collect_macos_sounds() -> Vec<(String, String)> {
    use std::collections::HashMap;

    let mut map: HashMap<String, (String, String)> = HashMap::new(); // key: lower name, value: (display, path)

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(home) = env::var("HOME") {
        dirs.push(Path::new(&home).join("Library/Sounds"));
    }
    dirs.push(PathBuf::from("/Library/Sounds"));
    dirs.push(PathBuf::from("/System/Library/Sounds"));

    for dir in dirs {
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_ascii_lowercase)
                    .unwrap_or_default();
                // Common macOS sound extensions
                if !matches!(ext.as_str(), "aiff" | "aif" | "caf" | "m4a" | "wav" | "mp3") {
                    continue;
                }
                let stem = match path.file_stem().and_then(|s| s.to_str()) {
                    Some(s) => s.to_string(),
                    None => continue,
                };
                let key = stem.to_ascii_lowercase();
                let abs = path.to_string_lossy().to_string();
                map.entry(key).or_insert((stem, abs));
            }
        }
    }

    let mut entries: Vec<(String, String)> = map.into_values().collect();
    entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    entries
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
pub(crate) fn list_macos_sound_names() -> Vec<String> {
    Vec::new()
}
#[cfg(not(target_os = "macos"))]
pub(crate) fn resolve_macos_sound_path(_name: &str) -> Option<String> {
    None
}

fn run_quiet(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) last_stdout_line: String,
    pub(crate) status: ExitStatus,
}

enum SummaryRequest {
    Interval(String),
    Final(String),
}

enum StreamPacket {
    StdoutChunk(String),
    StderrChunk(String),
    StdoutClosed,
    StderrClosed,
}

struct AggregatedOutput {
    stdout: String,
    stderr: String,
    last_stdout_line: String,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn run_codex(args: &[&str], prompt: &str) -> Result<CommandOutput> {
    // Prepare environment for codex: prepend our tool wrappers (e.g., cargo wrapper)
    let mut codex_cmd = Command::new("codex");

    if let Ok(cwd) = env::current_dir() {
        let wrapper_dir = cwd.join(".blueprints").join("bin");
        if wrapper_dir.exists() {
            // Best-effort: ensure wrappers are executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let cargo_wrapper = wrapper_dir.join("cargo");
                if let Ok(meta) = fs::metadata(&cargo_wrapper) {
                    let mode = meta.permissions();
                    let current = mode.mode();
                    // rwxr-xr-x (755)
                    let desired = (current & 0o666) | 0o111 | 0o644; // ensure exec bits
                    if current & 0o111 == 0
                        && let Ok(()) =
                            fs::set_permissions(&cargo_wrapper, PermissionsExt::from_mode(desired))
                    {
                        // set ok
                    }
                }
            }

            // Prepend wrapper path to PATH for codex child only
            if let Some(old_path) = env::var_os("PATH") {
                let sep = if cfg!(windows) { ";" } else { ":" };
                let new_path = format!(
                    "{}{}{}",
                    wrapper_dir.display(),
                    sep,
                    PathBuf::from(old_path).display()
                );
                codex_cmd.env("PATH", new_path);
            } else {
                codex_cmd.env("PATH", wrapper_dir.display().to_string());
            }

            // Expose the real cargo path so the wrapper can delegate without recursion
            if let Some(real_cargo) = resolve_in_path("cargo") {
                codex_cmd.env("BLUEPRINTS_REAL_CARGO", real_cargo);
            }
        }
    }

    let mut child = codex_cmd
        .args(args)
        .arg(prompt)
        .arg("--skip-git-repo-check")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn codex CLI")?;

    let stdout = child
        .stdout
        .take()
        .context("codex stdout pipe unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("codex stderr pipe unavailable")?;

    let do_summarize = summarize_enabled();

    let (summary_sender, summary_receiver) = if do_summarize {
        let (tx, rx) = mpsc::channel::<SummaryRequest>();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    let (stream_tx, stream_rx) = mpsc::channel::<StreamPacket>();

    let summarizer_handle = summary_receiver.map(|summary_rx| {
        thread::spawn(move || -> Result<()> {
            while let Ok(request) = summary_rx.recv() {
                let (chunk, final_update) = match request {
                    SummaryRequest::Interval(chunk) => (chunk, false),
                    SummaryRequest::Final(chunk) => (chunk, true),
                };

                if chunk.trim().is_empty() {
                    continue;
                }

                let summary = summarize_chunk(&chunk, final_update)?;
                if summary.trim().is_empty() {
                    continue;
                }

                if final_update {
                    log_codex(format!("Final update: {}", summary.trim()));
                } else {
                    log_codex(summary.trim());
                }
                io::stdout().flush().ok();
            }

            Ok(())
        })
    });

    let summary_sender_for_aggregator = summary_sender.clone();

    let aggregator_handle = thread::spawn(move || -> Result<AggregatedOutput> {
        let summary_interval = Duration::from_secs(15);
        let mut last_summary = Instant::now();
        let mut chunk_buffer = String::new();
        let mut stdout_capture = String::new();
        let mut stderr_capture = String::new();
        let mut last_stdout_line = String::new();
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        let mut summary_sender = summary_sender_for_aggregator;

        if do_summarize {
            while !(stdout_closed && stderr_closed) {
                let remaining = summary_interval.saturating_sub(last_summary.elapsed());

                if remaining.is_zero() {
                    if let Some(tx) = summary_sender.as_ref()
                        && !chunk_buffer.trim().is_empty()
                    {
                        let chunk = std::mem::take(&mut chunk_buffer);
                        tx.send(SummaryRequest::Interval(chunk))
                            .map_err(|err| anyhow!(err))?;
                    } else {
                        log_codex("Codex agent still running; no new output in the last 15s.");
                        io::stdout().flush().ok();
                    }
                    last_summary = Instant::now();
                    continue;
                }

                match stream_rx.recv_timeout(remaining) {
                    Ok(StreamPacket::StdoutChunk(chunk)) => {
                        stdout_capture.push_str(&chunk);
                        let trimmed = chunk.trim_end_matches(&['\n', '\r'][..]);
                        last_stdout_line = trimmed.to_string();
                        chunk_buffer.push_str(&chunk);
                    }
                    Ok(StreamPacket::StderrChunk(chunk)) => {
                        stderr_capture.push_str(&chunk);
                        if !chunk.trim().is_empty() {
                            chunk_buffer.push_str("[stderr] ");
                            chunk_buffer.push_str(&chunk);
                            if !chunk.ends_with('\n') {
                                chunk_buffer.push('\n');
                            }
                        }
                    }
                    Ok(StreamPacket::StdoutClosed) => {
                        stdout_closed = true;
                    }
                    Ok(StreamPacket::StderrClosed) => {
                        stderr_closed = true;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Some(tx) = summary_sender.as_ref()
                            && !chunk_buffer.trim().is_empty()
                        {
                            let chunk = std::mem::take(&mut chunk_buffer);
                            tx.send(SummaryRequest::Interval(chunk))
                                .map_err(|err| anyhow!(err))?;
                        } else {
                            log_codex("Codex agent still running; no new output in the last 15s.");
                            io::stdout().flush().ok();
                        }
                        last_summary = Instant::now();
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        stdout_closed = true;
                        stderr_closed = true;
                    }
                }
            }

            if !chunk_buffer.trim().is_empty()
                && let Some(tx) = summary_sender.take()
            {
                tx.send(SummaryRequest::Final(chunk_buffer))
                    .map_err(|err| anyhow!(err))?;
            }
        } else {
            // Verbatim streaming mode: forward chunks immediately to stdout/stderr with no summaries
            while !(stdout_closed && stderr_closed) {
                match stream_rx.recv() {
                    Ok(StreamPacket::StdoutChunk(chunk)) => {
                        stdout_capture.push_str(&chunk);
                        let trimmed = chunk.trim_end_matches(&['\n', '\r'][..]);
                        last_stdout_line = trimmed.to_string();
                        chunk_buffer.push_str(&chunk);
                        // forward to stdout
                        let _ = io::stdout().write_all(chunk.as_bytes());
                        let _ = io::stdout().flush();
                    }
                    Ok(StreamPacket::StderrChunk(chunk)) => {
                        stderr_capture.push_str(&chunk);
                        if !chunk.trim().is_empty() {
                            chunk_buffer.push_str("[stderr] ");
                            chunk_buffer.push_str(&chunk);
                            if !chunk.ends_with('\n') {
                                chunk_buffer.push('\n');
                            }
                        }
                        // forward to stderr
                        let _ = io::stderr().write_all(chunk.as_bytes());
                        let _ = io::stderr().flush();
                    }
                    Ok(StreamPacket::StdoutClosed) => {
                        stdout_closed = true;
                    }
                    Ok(StreamPacket::StderrClosed) => {
                        stderr_closed = true;
                    }
                    Err(mpsc::RecvError) => {
                        stdout_closed = true;
                        stderr_closed = true;
                    }
                }
            }
        }

        Ok(AggregatedOutput {
            stdout: stdout_capture,
            stderr: stderr_capture,
            last_stdout_line,
        })
    });

    let stream_tx_stdout = stream_tx.clone();
    let stdout_thread = thread::spawn(move || -> io::Result<()> {
        let mut reader = BufReader::new(stdout);
        let mut buffer = String::new();

        loop {
            buffer.clear();
            let read = reader.read_line(&mut buffer)?;
            if read == 0 {
                break;
            }

            stream_tx_stdout
                .send(StreamPacket::StdoutChunk(buffer.clone()))
                .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        }

        stream_tx_stdout
            .send(StreamPacket::StdoutClosed)
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        Ok(())
    });

    let stream_tx_stderr = stream_tx.clone();
    let stderr_thread = thread::spawn(move || -> io::Result<()> {
        let mut reader = BufReader::new(stderr);
        let mut buffer = [0u8; 4096];

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }

            let chunk = String::from_utf8_lossy(&buffer[..read]).to_string();
            stream_tx_stderr
                .send(StreamPacket::StderrChunk(chunk))
                .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        }

        stream_tx_stderr
            .send(StreamPacket::StderrClosed)
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        Ok(())
    });

    drop(stream_tx);
    if let Some(summary_tx) = summary_sender {
        drop(summary_tx);
    }

    let status = child
        .wait()
        .context("failed to wait for codex CLI to exit")?;

    let stdout_join = stdout_thread
        .join()
        .map_err(|_| anyhow!("stdout reader thread panicked"))?;
    stdout_join.map_err(|err| anyhow!(err))?;

    let stderr_join = stderr_thread
        .join()
        .map_err(|_| anyhow!("stderr reader thread panicked"))?;
    stderr_join.map_err(|err| anyhow!(err))?;

    let aggregated = aggregator_handle
        .join()
        .map_err(|_| anyhow!("summarizer aggregator thread panicked"))??;

    if let Some(handle) = summarizer_handle {
        let summarizer_result = handle
            .join()
            .map_err(|_| anyhow!("summarizer thread panicked"))?;
        summarizer_result?;
    }

    if do_summarize && !status.success() && !aggregated.stderr.trim().is_empty() {
        let mut stderr_handle = io::stderr().lock();
        stderr_handle.write_all(aggregated.stderr.as_bytes())?;
        stderr_handle.flush().ok();
    }

    Ok(CommandOutput {
        stdout: aggregated.stdout,
        last_stdout_line: aggregated.last_stdout_line,
        status,
    })
}

fn summarize_chunk(chunk: &str, final_update: bool) -> Result<String> {
    let mut instructions = "Summarize the Codex agent activity for the user as a single concise sentence or short paragraph. Focus on concrete actions, omit control tokens, and do not use bullet points."
        .to_string();
    if final_update {
        instructions.push_str(" Treat this as the final update before the agent stops.");
    } else {
        instructions.push_str(" This is an interim progress update.");
    }

    let prompt = format!("{instructions}\n\n<output_chunk>\n{chunk}\n</output_chunk>");

    let output = Command::new("codex")
        .args(["exec", "--profile", "summarizer"])
        .arg(prompt)
        .arg("--skip-git-repo-check")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run codex summarizer")?;

    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        let message = if stderr_text.trim().is_empty() {
            format!(
                "summarizer codex exec failed (exit {})",
                describe_exit(output.status)
            )
        } else {
            format!(
                "summarizer codex exec failed (exit {})\n{}",
                describe_exit(output.status),
                stderr_text.trim()
            )
        };
        return Err(anyhow!(message));
    }

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let summary =
        extract_codex_reply(stdout_text.as_ref()).unwrap_or_else(|| stdout_text.trim().to_string());

    Ok(summary)
}

pub(crate) fn describe_exit(status: ExitStatus) -> String {
    status.code().map_or_else(
        || "terminated by signal".to_string(),
        |code| code.to_string(),
    )
}

fn parse_env_usize(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(value) => value
            .parse::<usize>()
            .with_context(|| format!("invalid {key} value: {value}")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(value)) => Err(anyhow!(
            "{key} contains invalid UTF-8: {}",
            value.to_string_lossy()
        )),
    }
}

fn extract_codex_reply(output: &str) -> Option<String> {
    let marker = "\ncodex\n";
    let idx = output.rfind(marker)?;
    let after = &output[idx + marker.len()..];

    let mut lines = Vec::new();
    let mut seen_content = false;

    for line in after.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() && !seen_content {
            continue;
        }

        if trimmed.starts_with("tokens used")
            || trimmed.starts_with("[CODEX]")
            || trimmed.starts_with("reasoning effort")
            || trimmed.starts_with("session id")
            || trimmed.starts_with("Finished in")
        {
            break;
        }

        lines.push(line.to_string());
        seen_content = true;
    }

    let summary = lines.join("\n").trim().to_string();
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
}

fn parse_env_f64(key: &str, default: f64) -> Result<f64> {
    match env::var(key) {
        Ok(value) => value
            .parse::<f64>()
            .with_context(|| format!("invalid {key} value: {value}")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(value)) => Err(anyhow!(
            "{key} contains invalid UTF-8: {}",
            value.to_string_lossy()
        )),
    }
}

// Resolve an executable name to an absolute path using the current process PATH
fn resolve_in_path(bin: &str) -> Option<String> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&candidate) {
                    let mode = meta.permissions().mode();
                    if mode & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(candidate.display().to_string());
        }
        // On Windows, try .exe
        if cfg!(windows) {
            let candidate_exe = dir.join(format!("{bin}.exe"));
            if candidate_exe.is_file() {
                return Some(candidate_exe.display().to_string());
            }
        }
    }
    None
}
