//! Headless stdout protocol and exit codes.
//!
//! Owns the machine-readable vocabulary consumed by the validator and the
//! golden files (`E-STDOUT-PROTOCOL`, `E-ONE-PRINTER`): the terminal
//! `OK`/`FAIL` lines, the `CAPTURE` builders, and the `SCREENS` lines
//! (ADR 0021). The verify-side exit codes, markers, and report builder live in
//! [`universe_core::headless`] and are re-exported below; detail lines
//! (`LEVEL`, `SNAPSHOT`, `JOURNEY`, `RATIO`, `PREVIEW`, `INVERSE`) are
//! rendered by `verify.rs` and `capture.rs` in the exact formats specified
//! in `docs/engineering.md`; changing any line is a decision (ADR) because
//! the validator and the golden files depend on it.
//!
//! Each binary owns its own usage line: this program accepts `--verify`,
//! `--capture <dir>`, and `--screens <dir>`.

pub(crate) use universe_core::headless::{EXIT_FAIL, EXIT_PASS, EXIT_USAGE};

/// One-line usage for a wrong command line (stderr, exit 2).
///
/// Contains none of the validator's reserved words.
#[must_use]
pub(crate) const fn usage() -> &'static str {
    "usage: universe-app [--verify] [--capture <dir>] [--screens <dir>]"
}

/// `CAPTURE-OK files=<n> dir=<dir>` line.
#[must_use]
pub(crate) fn capture_ok(files: usize, dir: &str) -> String {
    format!("CAPTURE-OK files={files} dir={dir}")
}

/// `CAPTURE-FAIL files=<n> dir=<dir>` line.
#[must_use]
pub(crate) fn capture_fail(files: usize, dir: &str) -> String {
    format!("CAPTURE-FAIL files={files} dir={dir}")
}

/// `SCREENS-OK files=<n> dir=<dir>` line (ADR 0021).
#[must_use]
pub(crate) fn screens_ok(files: usize, dir: &str) -> String {
    format!("SCREENS-OK files={files} dir={dir}")
}

/// `SCREENS-FAIL <reason>` line (ADR 0021).
#[must_use]
pub(crate) fn screens_fail(reason: &str) -> String {
    format!("SCREENS-FAIL {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_lines_are_exact() {
        assert_eq!(screens_ok(7, "out"), "SCREENS-OK files=7 dir=out");
        assert_eq!(
            screens_fail("folder out not writable"),
            "SCREENS-FAIL folder out not writable"
        );
    }

    #[test]
    fn usage_names_every_flag() {
        for flag in ["--verify", "--capture <dir>", "--screens <dir>"] {
            assert!(usage().contains(flag), "usage lacks {flag}");
        }
    }
}
