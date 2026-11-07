//! Real headless catalog CLI: unchanged generation, explicit provenance and bounds.
//! No window, profile or new golden file is needed (#1239).

use std::process::{Command, Output};

fn run(args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_universe-app"))
        .args(args)
        .output()
}

#[test]
fn bounded_catalog_is_repeatable_and_boundary_data_is_not_a_generated_moon() {
    let args = ["--list-moons", "42", "root", "16", "64"];
    let first = run(&args).unwrap();
    assert!(first.status.success(), "{:?}", first);
    assert!(first.stderr.is_empty());
    let second = run(&args).unwrap();
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let text = String::from_utf8(first.stdout).unwrap();
    let mut count = 0;
    for line in text.lines() {
        if line.starts_with("MOON-BOUNDARY\t") {
            assert!(line.contains("origin=boundary-only"));
            assert!(line.contains("generated_identity=none"));
            assert!(line.contains("min_inclusive=true"));
            assert!(line.contains("end_inclusive=false"));
            assert!(!line.contains("\tmoon="));
        } else if line.starts_with("MOON\t") {
            count += 1;
            for key in [
                "seed",
                "planet",
                "system",
                "moon",
                "type",
                "radius_km",
                "orbit_km",
                "property_source",
            ] {
                assert!(line.contains(&format!("\t{key}=")), "{line}");
            }
            assert!(line.contains("\tplacement=none\t"));
            assert!(line.contains("\tcontact=unavailable:not-a-contact-observation"));
        }
    }
    assert!(count > 0 && count <= 64);
    assert!(text.ends_with(&format!(
        "MOONS-OK count={count} planet_limit=16 moon_limit=64 scope=bounded-not-global-minimum\n"
    )));
}

#[test]
fn nonexistent_portal_fails_and_malformed_cli_is_usage_not_another_context() {
    let absent = run(&["--list-moons", "42", "4294967295", "1", "1"]).unwrap();
    assert_eq!(absent.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(absent.stdout).unwrap(),
        "MOONS-FAIL not an existing portal prefix\n"
    );
    assert!(absent.stderr.is_empty());
    for args in [
        &["--list-moons", "42"][..],
        &["--list-moons", "42", "root", "0", "1"],
        &["--list-moons", "--size", "900x600"],
    ] {
        let malformed = run(args).unwrap();
        assert_eq!(malformed.status.code(), Some(2));
        assert!(malformed.stdout.is_empty());
        assert!(
            String::from_utf8(malformed.stderr)
                .unwrap()
                .starts_with("usage: universe-app")
        );
    }
}
