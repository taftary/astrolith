//! Headless `--verify` driver: print the shared report, exit with its code.
//!
//! The orchestration lives in [`universe_core::headless`] so every gate
//! binary shares one implementation (Q6). Imports nothing from
//! `universe-render` or `bevy`: everything the gate needs comes from
//! `universe-core` (the dependency-direction CI step asserts this).

use universe_core::headless::verify_report;

/// Replays the autopilot journey, checks every opened cell, prints the
/// shared headless report.
///
/// Returns the process exit code: 0 when everything passes, 1 otherwise.
#[must_use]
pub(crate) fn run_verify() -> i32 {
    let (report, code) = verify_report();
    print!("{report}");
    code
}
