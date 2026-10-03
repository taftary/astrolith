//! Headless stdout protocol and exit codes.
//!
//! Owns the machine-readable vocabulary consumed by the validator and the
//! golden files (`E-STDOUT-PROTOCOL`, `E-ONE-PRINTER`): exit codes, the
//! usage line, and the terminal `OK`/`FAIL` lines. Detail lines (`LEVEL`,
//! `SNAPSHOT`, `JOURNEY`, `RATIO`, `PREVIEW`, `INVERSE`) are printed by
//! `verify.rs` and `capture.rs` in the exact formats specified in
//! `docs/engineering.md`; changing any line is a decision (ADR) because
//! the validator and the golden files depend on it.

/// Exit code: every check passed.
pub(crate) const EXIT_PASS: i32 = 0;

/// Exit code: a check failed.
pub(crate) const EXIT_FAIL: i32 = 1;

/// Exit code: the command line itself was wrong (Q7).
pub(crate) const EXIT_USAGE: i32 = 2;

/// Machine-protocol marker: every headless check passed.
pub(crate) const VERIFY_OK: &str = "VERIFY-OK";

/// Machine-protocol marker: a headless check failed.
pub(crate) const VERIFY_FAIL: &str = "VERIFY-FAIL";

/// One-line usage for a wrong command line (stderr, exit 2).
///
/// Contains none of the validator's reserved words.
#[must_use]
pub(crate) const fn usage() -> &'static str {
    "usage: universe-app [--verify] [--capture <dir>]"
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
