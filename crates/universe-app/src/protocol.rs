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
//! `--capture <dir>`, `--screens <dir> [groups | --shots <file>]` (#847,
//! #863, ADR 0043), `--patch-bench` (#994) and `--route <file> <dir>` (#1022
//! T2), the window runs optionally followed by `--build-workers <n>`,
//! `--size <w>x<h>` and `--scale <f>`.

pub(crate) use universe_core::headless::{EXIT_FAIL, EXIT_PASS, EXIT_USAGE};

/// One-line usage for a wrong command line (stderr, exit 2).
///
/// Contains none of the validator's reserved words.
#[must_use]
pub(crate) const fn usage() -> &'static str {
    "usage: universe-app [--verify] [--capture <dir>] [--screens <dir> [groups | --shots <file> [--timing]]] [--patch-bench] [--list-worlds] [--list-moons [<seed> <prefix|root> <planets> <moons>]] [--list-water <chain>] [--route <file> <dir>] [--scenario <file> <new-dir> [--loading]] [--scenario-profile <file> <new-dir> <profile> [--loading]] [--build-workers <n>] [--size <w>x<h>] [--scale <f>]"
}

/// Actual generated facts, never a collision/landing observation (#1238).
#[must_use]
pub(crate) fn moon_line(moon: &universe_core::moon_catalog::ListedMoon) -> String {
    let chain = |parts: &[u32]| {
        parts
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".")
    };
    let host = moon.identity.planet();
    format!(
        "MOON\tseed={}\tplanet={}\tsystem={}\tmoon={}\ttype={}\tradius_km={:.17e}\torbit_km={:.17e}\tperiod_days={:.17e}\tpaint_seed={}\tevidence=observed-kind\tinstance_origin={}\tterrain_origin={}\tproperty_origin={}\tproperty_source={}\tplacement=none\tcontact=unavailable:not-a-contact-observation",
        host.seed().0,
        chain(host.chain()),
        chain(host.system()),
        moon.identity.marker().0,
        moon.kind.name(),
        moon.info.radius_km,
        moon.info.orbit_km,
        moon.info.period_days,
        moon.paint_seed,
        moon.origin.name(),
        moon.terrain_origin(),
        moon.property_origin(),
        moon.property_source(),
    )
}

/// Exact radius domain data is deliberately not given a generated identity.
#[must_use]
pub(crate) fn moon_boundary() -> String {
    use universe_core::moon_catalog::{SAMPLED_RADIUS_END_KM, SAMPLED_RADIUS_MIN_KM};
    format!(
        "MOON-BOUNDARY\torigin=boundary-only\tradius_min_km={SAMPLED_RADIUS_MIN_KM:.17e}\tmin_inclusive=true\tradius_end_km={SAMPLED_RADIUS_END_KM:.17e}\tend_inclusive=false\tgenerated_identity=none\tsource=terrain.rs:TerrainSampler::generate/MOON_STREAM_TAG"
    )
}

/// Complete real-input scenario, emitted only after evidence is flushed.
#[must_use]
pub(crate) const fn scenario_ok() -> &'static str {
    "SCENARIO-OK"
}

/// Non-successful real-input scenario, including incomplete runs.
#[must_use]
pub(crate) fn scenario_fail(reason: &str) -> String {
    format!("SCENARIO-FAIL {reason}")
}

/// `PATCHBENCH patches=<n> passes=<p> ms_per_patch=<ms>
/// cold_ms_per_patch=<ms>` line (#994): the median pass over the fixed set,
/// per patch, on one thread, with the parents' fine heights at hand (as a
/// refining build) and without them.
#[must_use]
pub(crate) fn patch_bench_line(
    patches: usize,
    passes: usize,
    warm_ms: f64,
    cold_ms: f64,
) -> String {
    format!(
        "PATCHBENCH patches={patches} passes={passes} ms_per_patch={warm_ms:.3} cold_ms_per_patch={cold_ms:.3}"
    )
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

/// `ROUTE-OK frames=<n> dir=<dir>` line (#1022 T2).
#[must_use]
pub(crate) fn route_ok(frames: usize, dir: &str) -> String {
    format!("ROUTE-OK frames={frames} dir={dir}")
}

/// `ROUTE-FAIL <reason>` line (#1022 T2).
#[must_use]
pub(crate) fn route_fail(reason: &str) -> String {
    format!("ROUTE-FAIL {reason}")
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
    fn patch_bench_line_is_exact() {
        assert_eq!(
            patch_bench_line(60, 5, 3.2104, 6.4567),
            "PATCHBENCH patches=60 passes=5 ms_per_patch=3.210 cold_ms_per_patch=6.457"
        );
    }

    #[test]
    fn usage_names_every_flag() {
        for flag in [
            "--verify",
            "--capture <dir>",
            "--screens <dir> [groups",
            "--shots <file>",
            "--patch-bench",
            "--route <file> <dir>",
            "--build-workers <n>",
            "--size <w>x<h>",
            "--scale <f>",
        ] {
            assert!(usage().contains(flag), "usage lacks {flag}");
        }
    }
}
