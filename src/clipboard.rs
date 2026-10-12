use anyhow::{anyhow, Result};
use std::{
    env,
    ffi::OsString,
    io::Write,
    process::{Command, Stdio},
};

const HELPERS: [&str; 2] = ["wl-copy", "clip.exe"];

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let native = has_display(env::var_os("DISPLAY"), env::var_os("WAYLAND_DISPLAY"))
        && arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .is_ok();
    if native || HELPERS.iter().any(|tool| pipe_to(tool, text).is_ok()) {
        return Ok(());
    }
    Err(anyhow!(
        "no clipboard: no X11 or Wayland display, and none of {} is installed",
        HELPERS.join(", ")
    ))
}

fn has_display(display: Option<OsString>, wayland: Option<OsString>) -> bool {
    display.is_some() || wayland.is_some()
}

fn pipe_to(program: &str, text: &str) -> std::io::Result<()> {
    let mut child = Command::new(program).stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(text.as_bytes())?;
    match child.wait()?.success() {
        true => Ok(()),
        false => Err(std::io::Error::other(format!("{program} failed"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_display_variable_enables_the_clipboard() {
        assert!(has_display(Some(OsString::from(":0")), None));
        assert!(has_display(None, Some(OsString::from("wayland-0"))));
        assert!(has_display(
            Some(OsString::from(":0")),
            Some(OsString::from("w"))
        ));
    }

    #[test]
    fn no_display_variable_means_no_clipboard() {
        assert!(!has_display(None, None));
    }

    #[test]
    fn piping_to_a_missing_helper_fails() {
        assert!(pipe_to("cmdpal-no-such-clipboard-helper", "text").is_err());
    }
}
