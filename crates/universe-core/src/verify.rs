//! Headless verification helpers for the automated checks.
//!
//! Spec v4 acceptance criteria 3 (determinism) and 5 (border agreement) plus
//! the headless `--verify` mode all build on these two predicates:
//! [`check_determinism`] regenerates a place twice and byte-compares the
//! canonical [`snapshot_generated`](crate::snapshot::snapshot_generated)
//! text, while [`check_border`] compares the shared-face samples of two
//! adjacent cells. Both are pure and need no window.

use crate::r#gen::{Constraints, Generator};
use crate::snapshot::snapshot_generated;

/// Regenerates a place twice and byte-compares the canonical snapshot.
///
/// Returns `true` when `generator.generate(seed, constraints)` snapshot text
/// is byte-identical across two runs. Comparing the canonical snapshot
/// (rather than `PartialEq` or `Debug`) proves what the validator actually
/// reads — the dumped bytes — is stable, including float formatting.
pub fn check_determinism<G: Generator>(
    generator: &G,
    seed: u64,
    constraints: &Constraints,
) -> bool {
    let first = generator.generate(seed, constraints);
    let second = generator.generate(seed, constraints);
    snapshot_generated(&first).into_bytes() == snapshot_generated(&second).into_bytes()
}

/// Compares the shared-border samples of two adjacent cells.
///
/// `a_samples` and `b_samples` are the positions sampled on the shared face,
/// in matching order, expressed in a common frame (the caller mirrors one
/// side where the face orientation requires it). Returns `true` when the two
/// slices have equal length and every paired component differs by at most
/// `epsilon`. A negative, non-finite, or `NaN` tolerance never passes, and
/// length mismatch never passes; two empty sample sets agree vacuously.
pub fn check_border(a_samples: &[[f64; 3]], b_samples: &[[f64; 3]], epsilon: f64) -> bool {
    if !epsilon.is_finite() || epsilon < 0.0 {
        return false;
    }
    if a_samples.len() != b_samples.len() {
        return false;
    }
    a_samples.iter().zip(b_samples.iter()).all(|(a, b)| {
        a.iter()
            .zip(b.iter())
            .all(|(x, y)| (*x - *y).abs() <= epsilon)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::UniformGenerator;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    #[test]
    fn determinism_check_passes_for_pure_generator() {
        let generator = UniformGenerator::new(32);
        assert!(check_determinism(&generator, 1234, &parent_constraints()));
        assert!(check_determinism(&generator, 0, &parent_constraints()));
        assert!(check_determinism(
            &generator,
            u64::MAX,
            &parent_constraints()
        ));
    }

    #[test]
    fn border_check_accepts_matching_and_rejects_drift() {
        let face = [[0.5, 0.1, -0.2], [0.5, -0.3, 0.0], [0.5, 0.0, 0.4]];
        assert!(check_border(&face, &face, 0.0));
        assert!(check_border(&face, &face, 1e-9));
        let drifted = [[0.5, 0.1, -0.2], [0.5, -0.3, 0.0], [0.5, 0.0, 0.9]];
        assert!(!check_border(&face, &drifted, 1e-9));
        // Small jitter within tolerance still agrees.
        let jittered = [
            [0.5, 0.100_000_000_1, -0.2],
            [0.5, -0.3, 0.0],
            [0.5, 0.0, 0.4],
        ];
        assert!(check_border(&face, &jittered, 1e-9));
    }

    #[test]
    fn border_check_rejects_mismatched_or_invalid_input() {
        let face = [[0.5, 0.0, 0.0]];
        assert!(!check_border(&face, &[], 1e-9));
        assert!(!check_border(&face, &face, -1.0));
        assert!(!check_border(&face, &face, f64::NAN));
        assert!(!check_border(&face, &face, f64::INFINITY));
        assert!(check_border(&[], &[], 0.0));
    }
}
