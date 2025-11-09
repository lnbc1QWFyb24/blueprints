use crate::logging::log_codex;
use anyhow::{Context, Result, anyhow};
use std::{
    collections::{HashSet, VecDeque},
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

pub(crate) fn set_summarize_enabled(enabled: bool) {
    let _ = SUMMARIZE_ENABLED.set(enabled);
}

fn summarize_enabled() -> bool {
    *SUMMARIZE_ENABLED.get_or_init(|| false)
}

const BLUEPRINTS_DIR_NAME: &str = "blueprints";
const IGNORED_SEARCH_DIRS: &[&str] = &[
    ".blueprints",
    ".direnv",
    ".git",
    ".idea",
    ".venv",
    ".vscode",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
];

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
    blueprints_dir: PathBuf,
}

impl BlueprintsContext {
    pub(crate) fn dir_token_value(&self) -> String {
        let dir_cow = self.blueprints_dir.to_string_lossy();
        if self.blueprints_dir.is_relative()
            && !dir_cow.starts_with("./")
            && !dir_cow.starts_with("../")
        {
            format!("./{dir_cow}")
        } else {
            dir_cow.into_owned()
        }
    }

    pub(crate) fn join(&self, file: &str) -> PathBuf {
        self.blueprints_dir.join(file)
    }
}

fn locate_blueprints_dir(workspace_root: &Path, module: &str) -> Option<PathBuf> {
    let module_path = Path::new(module);

    if let Some(found) = resolve_existing_dir(workspace_root, module_path.join(BLUEPRINTS_DIR_NAME))
    {
        return Some(found);
    }

    let module_leaf = module_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(module);

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(workspace_root.to_path_buf());
    visited.insert(workspace_root.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        let candidate = dir.join(module_leaf).join(BLUEPRINTS_DIR_NAME);
        if candidate.is_dir() {
            return Some(relativize_or_clone(workspace_root, candidate));
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }
            if entry.file_name().to_str().is_some_and(should_skip_dir) {
                continue;
            }
            let path = entry.path();
            if visited.insert(path.clone()) {
                queue.push_back(path);
            }
        }
    }

    resolve_existing_dir(workspace_root, PathBuf::from(BLUEPRINTS_DIR_NAME))
}

fn resolve_existing_dir(workspace_root: &Path, candidate: PathBuf) -> Option<PathBuf> {
    if candidate.is_absolute() {
        if candidate.is_dir() {
            Some(candidate)
        } else {
            None
        }
    } else {
        let abs = workspace_root.join(&candidate);
        if abs.is_dir() { Some(candidate) } else { None }
    }
}

pub(crate) fn relativize_or_clone(workspace_root: &Path, path: PathBuf) -> PathBuf {
    path.strip_prefix(workspace_root)
        .map(Path::to_path_buf)
        .unwrap_or(path)
}

// ---------- Module/crate target resolution ----------

/// Resolved target for implementation: a crate (package) and an optional module path inside it.
pub(crate) struct TargetSpec {
    pub(crate) workspace_root: PathBuf,
    pub(crate) crate_name: String,
    pub(crate) crate_root: PathBuf, // workspace-relative when possible
    pub(crate) module_rel: Option<PathBuf>, // relative to crate_root
}

/// Strictly resolve by crate/package name only. No path fallback.
pub(crate) fn resolve_target_from_crate(crate_name: &str) -> Result<TargetSpec> {
    let workspace_root = find_workspace_root()?;
    env::set_current_dir(&workspace_root).ok();
    if let Some((name, crate_root)) = enumerate_workspace_crates(&workspace_root)
        .into_iter()
        .find(|(name, _)| name == crate_name)
    {
        Ok(TargetSpec {
            workspace_root,
            crate_name: name,
            crate_root,
            module_rel: None,
        })
    } else {
        Err(anyhow!("crate '{crate_name}' not found in workspace"))
    }
}

/// Strictly resolve by module path only. The path must exist and be within a crate.
pub(crate) fn resolve_target_from_module_path(path: &str) -> Result<TargetSpec> {
    let workspace_root = find_workspace_root()?;
    env::set_current_dir(&workspace_root).ok();

    let arg_path = Path::new(path);
    let abs_path = if arg_path.is_absolute() {
        arg_path.to_path_buf()
    } else {
        workspace_root.join(arg_path)
    };

    if !abs_path.exists() {
        return Err(anyhow!("module path not found: {}", abs_path.display()));
    }
    let Some((crate_root_abs, crate_name)) = nearest_crate_root(&abs_path) else {
        return Err(anyhow!(
            "path '{}' is not inside a workspace crate",
            abs_path.display()
        ));
    };
    let crate_root_rel = relativize_or_clone(&workspace_root, crate_root_abs.clone());
    let module_rel = abs_path.strip_prefix(&crate_root_abs).ok().and_then(|p| {
        if p.as_os_str().is_empty() {
            None
        } else {
            Some(p.to_path_buf())
        }
    });

    Ok(TargetSpec {
        workspace_root,
        crate_name,
        crate_root: crate_root_rel,
        module_rel,
    })
}

/// Prefer crate-root/blueprints when present; otherwise fall back to discovery.
pub(crate) fn prepare_blueprints_for_crate(target: &TargetSpec) -> Result<BlueprintsContext> {
    // Ensure CWD is workspace root for path stability
    env::set_current_dir(&target.workspace_root).context("failed to switch to workspace root")?;

    let crate_root_abs = if target.crate_root.is_absolute() {
        target.crate_root.clone()
    } else {
        target.workspace_root.join(&target.crate_root)
    };
    let preferred = crate_root_abs.join(BLUEPRINTS_DIR_NAME);
    let blueprints_dir = if preferred.is_dir() {
        relativize_or_clone(&target.workspace_root, preferred)
    } else if let Some(found) = locate_blueprints_dir(&target.workspace_root, &target.crate_name) {
        found
    } else if Path::new(BLUEPRINTS_DIR_NAME).is_dir() {
        PathBuf::from(BLUEPRINTS_DIR_NAME)
    } else {
        // Fallback: keep a relative path at crate root for future creation
        relativize_or_clone(
            &target.workspace_root,
            crate_root_abs.join(BLUEPRINTS_DIR_NAME),
        )
    };

    Ok(BlueprintsContext { blueprints_dir })
}

/// Prefer module-root/blueprints when a module path is provided; otherwise fall back to crate-root.
pub(crate) fn prepare_blueprints_for_module(target: &TargetSpec) -> Result<BlueprintsContext> {
    // Ensure CWD is workspace root for path stability
    env::set_current_dir(&target.workspace_root).context("failed to switch to workspace root")?;

    let crate_root_abs = if target.crate_root.is_absolute() {
        target.crate_root.clone()
    } else {
        target.workspace_root.join(&target.crate_root)
    };

    // Determine the module root: if the provided module path is a file, use its parent; if a dir, use it.
    let module_root_abs = if let Some(rel) = &target.module_rel {
        let abs = crate_root_abs.join(rel);
        if abs.is_dir() {
            abs
        } else {
            abs.parent().map(Path::to_path_buf).unwrap_or(crate_root_abs)
        }
    } else {
        crate_root_abs
    };

    let preferred = module_root_abs.join(BLUEPRINTS_DIR_NAME);
    let blueprints_dir = relativize_or_clone(&target.workspace_root, preferred);

    Ok(BlueprintsContext { blueprints_dir })
}

/// Enumerate all workspace crates by scanning for Cargo.toml with a [package] name.
fn enumerate_workspace_crates(workspace_root: &Path) -> Vec<(String, PathBuf)> {
    let mut crates = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(workspace_root.to_path_buf());
    visited.insert(workspace_root.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file()
            && let Some(name) = read_package_name_from_manifest(&manifest)
        {
            crates.push((name, relativize_or_clone(workspace_root, dir.clone())));
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() || ft.is_symlink() {
                continue;
            }
            if entry.file_name().to_str().is_some_and(should_skip_dir) {
                continue;
            }
            let path = entry.path();
            if visited.insert(path.clone()) {
                queue.push_back(path);
            }
        }
    }

    crates
}

/// Walk upward from `start` to find the nearest directory containing a Cargo.toml with a [package] name.
fn nearest_crate_root(start: &Path) -> Option<(PathBuf, String)> {
    let mut cur = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        let manifest = cur.join("Cargo.toml");
        if manifest.is_file()
            && let Some(name) = read_package_name_from_manifest(&manifest)
        {
            return Some((cur, name));
        }
        if !(cur.pop()) {
            break;
        }
    }
    None
}

/// Minimal, line-oriented read of `[package] name = "..."` from a Cargo.toml manifest.
fn read_package_name_from_manifest(manifest: &Path) -> Option<String> {
    let content = fs::read_to_string(manifest).ok()?;
    let mut in_package = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim();
            if key == "name" {
                let mut val = line[idx + 1..].trim();
                if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    val = &val[1..val.len() - 1];
                }
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn should_skip_dir(name: &str) -> bool {
    IGNORED_SEARCH_DIRS
        .iter()
        .any(|ignored| ignored.eq_ignore_ascii_case(name))
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

    let (summary_sender_opt, summary_receiver_opt) = if do_summarize {
        let (tx, rx) = mpsc::channel::<SummaryRequest>();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    let (stream_tx, stream_rx) = mpsc::channel::<StreamPacket>();

    let summarizer_handle = summary_receiver_opt.map(|summary_rx| {
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

    let summary_tx_for_aggregator = summary_sender_opt.clone();

    let aggregator_handle = thread::spawn(move || -> Result<AggregatedOutput> {
        let summary_interval = Duration::from_secs(15);
        let mut last_summary = Instant::now();
        let mut chunk_buffer = String::new();
        let mut stdout_capture = String::new();
        let mut stderr_capture = String::new();
        let mut last_stdout_line = String::new();
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        let mut summary_tx = summary_tx_for_aggregator;

        if do_summarize {
            while !(stdout_closed && stderr_closed) {
                let remaining = summary_interval.saturating_sub(last_summary.elapsed());

                if remaining.is_zero() {
                    if let Some(tx) = summary_tx.as_ref()
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
                        if let Some(tx) = summary_tx.as_ref()
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
                && let Some(tx) = summary_tx.take()
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
    if let Some(summary_tx) = summary_sender_opt {
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
