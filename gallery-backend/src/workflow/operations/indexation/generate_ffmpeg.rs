use std::process::Command;
/// Creates a base `ffmpeg` command with flags to ensure it runs silently.
/// This prevents duplicating arguments and ensures all ffmpeg calls are quiet.
pub fn create_silent_ffmpeg_command() -> Command {
    let mut cmd = Command::new("ffmpeg");
    // These global options must come before the input/output options.
    cmd.args(["-v", "quiet", "-hide_banner", "-nostats", "-nostdin"]);
    cmd
}
