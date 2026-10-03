//! Golden baseline for `--verify` and `--capture` (M0 for #85).
//!
//! Two tests, no new dependencies:
//! - `verify_stdout_matches_golden`: runs the app with `--verify`, asserts
//!   exit 0, and byte-compares stdout against `tests/golden/verify.txt`.
//! - `capture_files_match_golden`: runs the app with `--capture <tmpdir>`,
//!   asserts exit 0, and compares the sorted `name fnv1a64` listing of every
//!   file in the capture dir against `tests/golden/capture.txt`.
//!
//! `UPDATE_GOLDEN=1` rewrites both golden files from the current binary
//! output. CI never sets it; the committed files are the baseline from M0.

use std::path::PathBuf;

/// Locate the `universe-app` binary under test.
///
/// Cargo exposes the binary to integration tests through
/// `CARGO_BIN_EXE_<name>`. The exact variable spelling differs by toolchain
/// (`-` versus `_`), so both are accepted.
fn app_bin() -> PathBuf {
    let bin =
        option_env!("CARGO_BIN_EXE_universe-app").or(option_env!("CARGO_BIN_EXE_universe_app"));
    assert!(
        bin.is_some(),
        "missing CARGO_BIN_EXE for universe-app (run via `cargo test`)"
    );
    PathBuf::from(bin.unwrap_or_default())
}

/// FNV-1a 64: offset basis 0xcbf29ce484222325, prime 0x100000001b3.
/// Implemented here so the test adds no dependency.
fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// True when the golden files should be rewritten from current output.
fn update_golden() -> bool {
    std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Report the first differing line between two byte buffers.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let a_lines: Vec<&[u8]> = a.split(|&c| c == b'\n').collect();
    let b_lines: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
    for (i, (x, y)) in a_lines.iter().zip(b_lines.iter()).enumerate() {
        if x != y {
            return format!(
                "first difference at line {}:\n  actual:   {}\n  expected: {}",
                i + 1,
                String::from_utf8_lossy(x.get(..300).unwrap_or(x)),
                String::from_utf8_lossy(y.get(..300).unwrap_or(y)),
            );
        }
    }
    if a_lines.len() != b_lines.len() {
        return format!(
            "same prefix, line count differs: actual {} vs expected {}",
            a_lines.len(),
            b_lines.len()
        );
    }
    format!(
        "byte length differs with identical lines: actual {} vs expected {}",
        a.len(),
        b.len()
    )
}

#[test]
fn verify_stdout_matches_golden() {
    let out = std::process::Command::new(app_bin())
        .arg("--verify")
        .output()
        .expect("run universe-app --verify");
    assert!(
        out.status.success(),
        "--verify exited with {}: stderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "--verify must not write to stderr, got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let golden_path = golden_dir().join("verify.txt");
    if update_golden() {
        std::fs::create_dir_all(golden_dir()).expect("create golden dir");
        std::fs::write(&golden_path, &out.stdout).expect("write verify.txt");
        return;
    }
    let expected = std::fs::read(&golden_path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {} (run with UPDATE_GOLDEN=1 to create it)",
            golden_path.display()
        )
    });
    assert!(
        out.stdout == expected,
        "--verify stdout differs from {} ({} vs {} bytes).\n{}",
        golden_path.display(),
        out.stdout.len(),
        expected.len(),
        first_diff(&out.stdout, &expected)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("VERIFY-OK"),
        "--verify output must end with VERIFY-OK"
    );
}

#[test]
fn capture_files_match_golden() {
    let dir = std::env::temp_dir().join(format!("astrolith-golden-capture-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear previous capture tmpdir");
    }
    std::fs::create_dir_all(&dir).expect("create capture tmpdir");
    let out = std::process::Command::new(app_bin())
        .arg("--capture")
        .arg(&dir)
        .output()
        .expect("run universe-app --capture");
    assert!(
        out.status.success(),
        "--capture exited with {}: stdout:\n{}stderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut entries: Vec<(String, u64)> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("list capture dir") {
        let entry = entry.expect("dir entry");
        if !entry.file_type().expect("file type").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let bytes = std::fs::read(entry.path()).expect("read capture file");
        entries.push((name, fnv1a64(&bytes)));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        !entries.is_empty(),
        "--capture produced no files in {}",
        dir.display()
    );
    let mut listing = String::new();
    for (name, hash) in &entries {
        listing.push_str(&format!("{name} {hash:016x}\n"));
    }
    let golden_path = golden_dir().join("capture.txt");
    if update_golden() {
        std::fs::create_dir_all(golden_dir()).expect("create golden dir");
        std::fs::write(&golden_path, listing.as_bytes()).expect("write capture.txt");
        return;
    }
    let expected = std::fs::read_to_string(&golden_path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {} (run with UPDATE_GOLDEN=1 to create it)",
            golden_path.display()
        )
    });
    assert!(
        listing == expected,
        "capture listing differs from {}.\nactual:\n{listing}expected:\n{expected}",
        golden_path.display()
    );
}
