//! Parity with the committed M0 baseline (read-only).
//!
//! Runs the `universe-verify` binary and byte-compares its `--verify`
//! stdout against `universe-app`'s golden file. The golden files belong to
//! the M0 baseline and are never rewritten from here.

use std::path::PathBuf;

/// Locates the `universe-verify` binary under test.
fn verify_bin() -> PathBuf {
    let bin = option_env!("CARGO_BIN_EXE_universe-verify")
        .or(option_env!("CARGO_BIN_EXE_universe_verify"));
    assert!(
        bin.is_some(),
        "missing CARGO_BIN_EXE for universe-verify (run via `cargo test`)"
    );
    PathBuf::from(bin.unwrap_or_default())
}

/// Committed M0 baseline written by `universe-app` (read-only here).
fn golden_verify_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../universe-app/tests/golden/verify.txt")
}

#[test]
fn stdout_matches_the_committed_baseline() {
    let out = std::process::Command::new(verify_bin())
        .arg("--verify")
        .output()
        .expect("run universe-verify --verify");
    assert!(
        out.status.success(),
        "--verify exited with {}: stderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let expected = std::fs::read(golden_verify_path()).expect("read M0 golden verify.txt");
    assert!(
        out.stdout == expected,
        "--verify stdout differs from the M0 baseline ({} vs {} bytes)",
        out.stdout.len(),
        expected.len()
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("VERIFY-OK"),
        "--verify output must end with VERIFY-OK"
    );
}

#[test]
fn unknown_flag_exits_2_with_usage() {
    let out = std::process::Command::new(verify_bin())
        .arg("--frobnicate")
        .output()
        .expect("run universe-verify with a bad flag");
    assert_eq!(
        out.status.code(),
        Some(2),
        "wrong command line must exit 2, got {}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("usage: universe-verify [--verify]"),
        "stderr must carry the usage line, got:\n{stderr}"
    );
}
