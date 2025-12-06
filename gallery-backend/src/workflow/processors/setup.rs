//! Setup/initialization module - handles application startup tasks
//!
//! Includes:
//! - FFmpeg/FFprobe availability check
//! - Folder structure initialization
//! - Logger initialization
//! - Cache file cleanup

use crate::public::tui::LOGGER_TX;
use env_logger::{Builder, WriteStyle};
use log::kv::Key;
use std::{fs, io::Write, process::Command};
use superconsole::style::Stylize;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// ────────────────────────────────────────────────────────────────
// FFmpeg Check
// ────────────────────────────────────────────────────────────────

/// Check if ffmpeg and ffprobe are available in PATH
pub fn check_ffmpeg_and_ffprobe() {
    for command in &["ffmpeg", "ffprobe"] {
        match Command::new(command).arg("-version").output() {
            Ok(output) if output.status.success() => {
                let version_info = String::from_utf8_lossy(&output.stdout);
                let version_number = version_info
                    .lines()
                    .next()
                    .unwrap_or("Unknown version")
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("Unknown");
                info!("{} version: {}", command, version_number);
            }
            Ok(_) => {
                error!(
                    "`{}` command was found, but it returned an error. Please ensure it's correctly installed.",
                    command
                );
            }
            Err(_) => {
                error!(
                    "`{}` is not installed or not available in PATH. Please install it before running the application.",
                    command
                );
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Folder Initialization
// ────────────────────────────────────────────────────────────────

/// Create required folder structure for the application
pub fn initialize_folder() {
    std::fs::create_dir_all("./db").unwrap();
    std::fs::create_dir_all("./object/imported").unwrap();
    std::fs::create_dir_all("./object/compressed").unwrap();
    std::fs::create_dir_all("./upload").unwrap();
}

// ────────────────────────────────────────────────────────────────
// Cache File Cleanup
// ────────────────────────────────────────────────────────────────

/// Clean up temporary cache files on startup
pub fn initialize_file() {
    {
        let db_path = "./db/temp_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear tree cache");
                }
                Err(_) => {
                    error!("Fail to delete cache data ./db/temp_db.redb")
                }
            }
        }
    }
    {
        let db_path = "./db/cache_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear query cache");
                }
                Err(_) => {
                    error!("Fail to delete cache data ./db/cache_db.redb")
                }
            }
        }
    }
    {
        let db_path = "./db/expire_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear expire table");
                }
                Err(_) => {
                    error!("Fail to delete expire table ./db/expire_db.redb")
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Logger Initialization
// ────────────────────────────────────────────────────────────────

/// A `Write` adapter that sends each incoming line over a Tokio channel.
pub struct TokioPipe(pub UnboundedSender<String>);

impl std::io::Write for TokioPipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Decode bytes into a UTF-8 string, replacing invalid sequences
        let s = String::from_utf8_lossy(buf);
        // Split on newline, replace tabs, and send non-empty lines
        for line in s.split_terminator('\n') {
            let clean = line.replace('\t', "    ");
            if !clean.is_empty() {
                let _ = self.0.send(clean.to_string());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // No buffering, so nothing to flush
        Ok(())
    }
}

/// Initialize the logger and return a receiver for formatted log lines.
pub fn initialize_logger() -> UnboundedReceiver<String> {
    // Create a channel and save the sender globally
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    LOGGER_TX.set(tx).unwrap();

    Builder::new()
        // Always include ANSI codes so StyledContent can reset itself
        .write_style(WriteStyle::Always)
        .format(|buf, record| {
            // Colorize timestamp in dark grey
            let ts = buf.timestamp().to_string().dark_grey();

            // Colorize level with default style (includes reset)
            let level_style = buf.default_level_style(record.level());
            let lvl = format!(
                "{}{}{}",
                level_style.render(),
                record.level(),
                level_style.render_reset()
            );

            // Colorize module target in dark grey
            let tgt = record.target().dark_grey();

            // Extract raw duration and format to 2 decimal places
            let dur_raw = record
                .key_values()
                .get(Key::from("duration"))
                .map(|v| {
                    let s = format!("{}", v);
                    if let Some(idx) = s.find(|c: char| c.is_alphabetic()) {
                        let (num, unit) = (&s[..idx], &s[idx..]);
                        if let Ok(val) = num.parse::<f32>() {
                            // Insert space between number and unit
                            return format!("{:.2} {}", val, unit);
                        }
                    }
                    s
                })
                .unwrap_or_default();

            // Right-align or pad the duration field to width 10, then color it cyan
            let dur = if dur_raw.is_empty() {
                " ".repeat(10)
            } else {
                format!("{:>10}", dur_raw).cyan().to_string()
            };

            // First, print the common prefix for all log entries
            writeln!(buf, "{} {} {}", ts, lvl, tgt)?;

            // Convert log message to string
            let message = format!("{}", record.args());

            // Calculate the indent for subsequent lines (duration width 10 + 1 space)
            let subsequent_indent = " ".repeat(11);

            // Split the message into lines
            let mut lines = message.lines();

            // Handle the first line of the message, prefix with duration
            if let Some(first_line) = lines.next() {
                writeln!(buf, "{} {}", dur, first_line)?;
            }

            // Handle all subsequent lines, indenting them properly
            for line in lines {
                writeln!(buf, "{}{}", subsequent_indent, line)?;
            }

            Ok(())
        })
        // Send formatted output through our custom pipe
        .target(env_logger::Target::Pipe(Box::new(TokioPipe(
            LOGGER_TX.get().unwrap().clone(),
        ))))
        // Only show INFO+ globally, WARN+ for Rocket
        .filter(None, log::LevelFilter::Info)
        .filter(Some("rocket"), log::LevelFilter::Warn)
        .init();

    rx
}

pub fn initialize() -> UnboundedReceiver<String> {
    let rx = initialize_logger();
    check_ffmpeg_and_ffprobe();
    initialize_folder();
    initialize_file();
    rx
}
