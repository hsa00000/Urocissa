use env_logger::{Builder, WriteStyle};
use log::kv::Key;
use superconsole::style::Stylize;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::public::tui::LOGGER_TX;

/// Startup logging must remain visible before the TUI exists. Once the TUI
/// task starts consuming the receiver, log lines are routed there instead.
static TUI_ATTACHED: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_tui_attached() {
    TUI_ATTACHED.store(true, Ordering::Release);
}

/// A `Write` adapter that sends each incoming line over a Tokio channel.
pub struct TokioPipe(pub UnboundedSender<String>);

impl std::io::Write for TokioPipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !TUI_ATTACHED.load(Ordering::Acquire) {
            let mut stderr = std::io::stderr().lock();
            stderr.write_all(buf)?;
            stderr.flush()?;
            return Ok(buf.len());
        }

        // Decode bytes into a UTF-8 string, replacing invalid sequences
        let s = String::from_utf8_lossy(buf);
        // Split on newline, replace tabs, and send non-empty lines
        for line in s.split_terminator('\n') {
            let clean = line.replace('\t', "    ");
            if !clean.is_empty() {
                let _ = self.0.send(clean.clone());
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
            crate::performance::record_log(record);
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
                    let s = format!("{v}");
                    if let Some(idx) = s.find(|c: char| c.is_alphabetic()) {
                        let (num, unit) = (&s[..idx], &s[idx..]);
                        if let Ok(val) = num.parse::<f32>() {
                            // Insert space between number and unit
                            return format!("{val:.2} {unit}");
                        }
                    }
                    s
                })
                .unwrap_or_default();

            // Right-align or pad the duration field to width 10, then color it cyan
            let dur = if dur_raw.is_empty() {
                " ".repeat(10)
            } else {
                format!("{dur_raw:>10}").cyan().to_string()
            };

            // First, print the common prefix for all log entries
            writeln!(buf, "{ts} {lvl} {tgt}")?;

            // Convert log message to string
            let message = format!("{}", record.args());

            // Calculate the indent for subsequent lines (duration width 10 + 1 space)
            let subsequent_indent = " ".repeat(11);

            // Split the message into lines
            let mut lines = message.lines();

            // Handle the first line of the message, prefix with duration
            if let Some(first_line) = lines.next() {
                writeln!(buf, "{dur} {first_line}")?;
            }

            // Handle all subsequent lines, indenting them properly
            for line in lines {
                writeln!(buf, "{subsequent_indent}{line}")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::sync::mpsc::error::TryRecvError;

    static TUI_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn startup_pipe_does_not_queue_lines_before_tui_attach() {
        let _guard = TUI_TEST_LOCK.lock().unwrap();
        TUI_ATTACHED.store(false, Ordering::Release);
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut pipe = TokioPipe(sender);

        pipe.write_all(b"startup line\n").unwrap();

        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn attached_pipe_queues_lines_for_tui() {
        let _guard = TUI_TEST_LOCK.lock().unwrap();
        TUI_ATTACHED.store(true, Ordering::Release);
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut pipe = TokioPipe(sender);

        pipe.write_all(b"attached line\n").unwrap();

        assert_eq!(receiver.try_recv().unwrap(), "attached line");
        TUI_ATTACHED.store(false, Ordering::Release);
    }
}
