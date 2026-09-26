use anyhow::Result;
use std::{io::Write, process::{Command, Stdio}};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let native_error = match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
        Ok(()) => return Ok(()),
        Err(error) => error,
    };
    // Wayland sessions and WSL have no reachable native clipboard, but ship these helpers.
    if ["wl-copy", "clip.exe"].iter().any(|tool| pipe_to(tool, text).is_ok()) {
        return Ok(());
    }
    Err(native_error.into())
}

fn pipe_to(program: &str, text: &str) -> std::io::Result<()> {
    let mut child = Command::new(program).stdin(Stdio::piped()).spawn()?;
    child.stdin.take().expect("stdin was piped").write_all(text.as_bytes())?;
    match child.wait()?.success() {
        true => Ok(()),
        false => Err(std::io::Error::other(format!("{program} failed"))),
    }
}
