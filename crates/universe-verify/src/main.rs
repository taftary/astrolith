//! Bevy-free headless gate: the same universe checks, no window layer.
//!
//! Prints exactly what `universe-app --verify` prints (both go through
//! [`universe_core::headless`]), so the automated gate stays fast while the
//! window binary keeps working unchanged. Bare invocation or `--verify`
//! runs the checks; anything else prints one usage line to stderr and
//! exits 2. Flag parsing uses `std::env` only.

use universe_core::headless::{EXIT_USAGE, verify_report};

/// One-line usage for a wrong command line (stderr, exit 2).
///
/// Contains none of the validator's reserved words.
fn usage() -> &'static str {
    "usage: universe-verify [--verify]"
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// Bare or `--verify`: run the headless gate.
    Verify,
    /// Anything else: print usage to stderr, exit 2.
    Usage,
}

/// Parses the arguments after the program name (pure: unit-tested below).
fn parse_args(args: &[String]) -> Action {
    match args {
        [] => Action::Verify,
        [verify] if verify == "--verify" => Action::Verify,
        _ => Action::Usage,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Action::Verify => {
            let (report, code) = verify_report();
            print!("{report}");
            std::process::exit(code);
        }
        Action::Usage => {
            eprintln!("{}", usage());
            std::process::exit(EXIT_USAGE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_string()).collect()
    }

    #[test]
    fn bare_invocation_runs_verify() {
        assert_eq!(parse_args(&[]), Action::Verify);
    }

    #[test]
    fn verify_flag_verifies() {
        assert_eq!(parse_args(&args(&["--verify"])), Action::Verify);
    }

    #[test]
    fn unknown_flag_is_usage() {
        assert_eq!(parse_args(&args(&["--frobnicate"])), Action::Usage);
    }

    #[test]
    fn extra_args_are_usage() {
        assert_eq!(
            parse_args(&args(&["--verify", "extra"])),
            Action::Usage
        );
    }

    #[test]
    fn usage_has_no_reserved_words() {
        for word in ["error", "panic", "warn", "deprecat", "FAILED"] {
            assert!(
                !usage().contains(word),
                "usage line contains reserved word {word}"
            );
        }
    }
}
