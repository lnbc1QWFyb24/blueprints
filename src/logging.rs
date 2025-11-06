use nu_ansi_term::Color;
use std::sync::OnceLock;
use time::{OffsetDateTime, UtcOffset, macros::format_description};

static LOCAL_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

pub fn init() {
    let offset = match UtcOffset::current_local_offset() {
        Ok(offset) => offset,
        Err(error) => {
            eprintln!(
                "unable to determine local timezone offset ({error}); falling back to UTC logs"
            );
            UtcOffset::UTC
        }
    };

    let _ = LOCAL_OFFSET.set(offset);
}

pub fn log_error(message: impl AsRef<str>) {
    emit(Color::Red, "ERROR", message.as_ref(), Output::Stderr);
}

pub fn log_blueprints(message: impl AsRef<str>) {
    emit(Color::Blue, "BLUEPRINTS", message.as_ref(), Output::Stderr);
}

pub fn log_codex(message: impl AsRef<str>) {
    emit(
        Color::Rgb(128, 128, 128),
        "CODEX",
        message.as_ref(),
        Output::Stdout,
    );
}

#[derive(Copy, Clone)]
enum Output {
    Stdout,
    Stderr,
}

fn emit(color: Color, label: &str, message: &str, output: Output) {
    let ts = timestamp();
    let line = format!("[{label}][{ts}] - {message}");
    let painted = color.paint(line);

    match output {
        Output::Stdout => println!("{painted}"),
        Output::Stderr => eprintln!("{painted}"),
    }
}

fn timestamp() -> String {
    let offset = *LOCAL_OFFSET.get_or_init(determine_offset);
    let now = OffsetDateTime::now_utc().to_offset(offset);
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    now.format(&format)
        .unwrap_or_else(|_| "unknown".to_string())
}

fn determine_offset() -> UtcOffset {
    UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC)
}
