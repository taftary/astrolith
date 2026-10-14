//! Text to the system clipboard through the platform's own command (#665).
//!
//! The lab adds no clipboard crate: bevy_egui's clipboard feature pulls
//! `arboard`, whose Windows backend is under a licence `deny.toml` does not
//! allow (`E-DENY`). The platforms already ship a command that reads the
//! text on its standard input.

use std::io::Write;
use std::process::{Command, Stdio};

/// The commands tried in order on this platform, with their arguments.
#[must_use]
pub fn commands() -> &'static [(&'static str, &'static [&'static str])] {
    if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else {
        &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])]
    }
}

/// Puts `text` on the clipboard.
///
/// # Errors
///
/// A short message for the status bar when no command took the text.
pub fn copy(text: &str) -> Result<(), String> {
    for (program, args) in commands() {
        let Ok(mut child) = Command::new(program)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let written = child
            .stdin
            .take()
            .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
        if written && child.wait().is_ok_and(|status| status.success()) {
            return Ok(());
        }
    }
    let names: Vec<&str> = commands().iter().map(|(program, _)| *program).collect();
    Err(format!("no clipboard: {} not found", names.join(" or ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_desktop_has_a_clipboard_command() {
        let list = commands();
        assert!(!list.is_empty());
        if cfg!(target_os = "windows") {
            assert_eq!(list[0].0, "clip");
        }
    }

    /// Run by hand on the dev PC (`cargo test -p engine-lab -- --ignored`):
    /// the text really reaches the Windows clipboard.
    #[test]
    #[ignore = "touches the real clipboard"]
    fn the_text_reaches_the_clipboard_on_this_pc() {
        let text = "pub const SQUARE_M: f64 = 1000.0;";
        copy(text).expect("copied");
        if cfg!(target_os = "windows") {
            let out = Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-Clipboard"])
                .output()
                .expect("powershell");
            assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), text);
        }
    }
}
