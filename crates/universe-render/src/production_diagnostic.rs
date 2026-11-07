//! Optional bounded witness of production body integration (#1238).
//!
//! Normal startup never creates this resource. Nothing here can select a fixture,
//! set a body/clock, change a gameplay decision or supply missing ground.

use bevy::prelude::*;
use universe_core::body::Body;
use universe_core::frame_diagnostic::{FramePath, FrameStepDiagnostic, FrozenConversion};

/// The physical inputs/results of the most recent ordinary body step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BodyStepRecord {
    pub seed: u64,
    pub open_seed: u64,
    pub elapsed_s: f64,
    pub game_s: f64,
    pub before: Body,
    pub after: Body,
    pub gravity_cell_per_s2: [f64; 3],
    pub strongest_m_per_s2: f64,
}

/// Fixed-size last-step and lifetime conversion counters, not a growing history.
#[derive(Resource, Debug, Default)]
pub(crate) struct ProductionDiagnostic {
    pub step: Option<BodyStepRecord>,
    pub frame: FrameStepDiagnostic,
    pub sequence: u64,
    pub entered: u64,
    pub reanchored: u64,
    pub left: u64,
    pub max_position_m: Option<f64>,
    pub max_direction_deg: Option<f64>,
}

impl ProductionDiagnostic {
    pub(crate) fn begin(&mut self) {
        self.step = None;
        self.frame = default();
    }

    pub(crate) fn finish(&mut self, record: BodyStepRecord) {
        self.step = Some(record);
        self.sequence = self.sequence.saturating_add(1);
        match self.frame.path {
            FramePath::Entered => self.entered = self.entered.saturating_add(1),
            FramePath::Reanchored => self.reanchored = self.reanchored.saturating_add(1),
            FramePath::Left => self.left = self.left.saturating_add(1),
            _ => (),
        }
        for witness in [self.frame.input, self.frame.output].into_iter().flatten() {
            let position = witness.cell_round_trip.0.max(witness.local_round_trip.0);
            self.max_position_m = Some(
                self.max_position_m
                    .map_or(position, |old| old.max(position)),
            );
            self.max_direction_deg = Some(
                self.max_direction_deg
                    .map_or(witness.direction_error.0, |old| {
                        old.max(witness.direction_error.0)
                    }),
            );
        }
    }
}

fn witness(
    snapshot: &mut crate::scenario_run::Snapshot,
    key: &str,
    witness: Option<FrozenConversion>,
    cell: &str,
    local: &str,
) {
    use crate::scenario_values::{unavailable, value, vector};
    let Some(w) = witness else {
        for suffix in [
            "cell_error_m",
            "local_error_m",
            "direction_error_deg",
            "cell_before",
            "cell_after",
            "local",
            "forward_before",
            "forward_after",
            "local_forward",
            "comparison",
        ] {
            unavailable(
                snapshot,
                &format!("{key}.{suffix}"),
                "no-finite-production-local-conversion",
            );
        }
        return;
    };
    value(
        snapshot,
        &format!("{key}.comparison"),
        "one-frozen-pose-not-consecutive-frames",
    );
    value(
        snapshot,
        &format!("{key}.cell_error_m"),
        w.cell_round_trip.0,
    );
    value(
        snapshot,
        &format!("{key}.local_error_m"),
        w.local_round_trip.0,
    );
    value(
        snapshot,
        &format!("{key}.direction_error_deg"),
        w.direction_error.0,
    );
    vector(
        snapshot,
        &format!("{key}.cell_before"),
        w.cell_before.0,
        cell,
    );
    vector(snapshot, &format!("{key}.cell_after"), w.cell_after.0, cell);
    vector(snapshot, &format!("{key}.local"), w.encoded_local.0, local);
    vector(
        snapshot,
        &format!("{key}.forward_before"),
        w.forward_before.0,
        &cell.replace("open-cell-units", "direction"),
    );
    vector(
        snapshot,
        &format!("{key}.forward_after"),
        w.forward_after.0,
        &cell.replace("open-cell-units", "direction"),
    );
    vector(
        snapshot,
        &format!("{key}.local_forward"),
        w.encoded_forward.0,
        &local.replace("metres", "direction"),
    );
}

pub(crate) fn observe(world: &World, snapshot: &mut crate::scenario_run::Snapshot) {
    use crate::scenario_values::{unavailable, value, vector};
    let Some(diagnostic) = world.get_resource::<ProductionDiagnostic>() else {
        unavailable(snapshot, "conversion.path", "diagnostic-not-requested");
        return;
    };
    value(snapshot, "conversion.sequence", diagnostic.sequence);
    value(snapshot, "conversion.entered_total", diagnostic.entered);
    value(
        snapshot,
        "conversion.reanchored_total",
        diagnostic.reanchored,
    );
    value(snapshot, "conversion.left_total", diagnostic.left);
    for (key, maximum) in [
        ("conversion.max_position_m", diagnostic.max_position_m),
        ("conversion.max_direction_deg", diagnostic.max_direction_deg),
    ] {
        match maximum {
            Some(v) => value(snapshot, key, v),
            None => unavailable(snapshot, key, "no-production-conversion-observed"),
        }
    }
    let universe = world.resource::<crate::Universe>();
    let current = diagnostic.step.filter(|step| {
        step.seed == universe.root.0
            && step.open_seed == universe.open_seed()
            && step.elapsed_s == world.resource::<Time>().elapsed_secs_f64()
    });
    let Some(step) = current else {
        unavailable(
            snapshot,
            "conversion.path",
            "no-body-step-in-this-frame-and-context",
        );
        for key in [
            "step.body_before",
            "step.body_after",
            "step.gravity_m_per_s2",
            "step.strongest_gravity_m_per_s2",
        ] {
            unavailable(snapshot, key, "no-body-step-in-this-frame-and-context");
        }
        witness(snapshot, "conversion.input", None, "", "");
        witness(snapshot, "conversion.output", None, "", "");
        return;
    };
    let identity = format!(
        "seed={}/path={}",
        step.seed,
        crate::scenario_setup::chain_text(&universe.path.indices())
    );
    let cell = format!("{identity}/open-cell-units");
    let local = format!(
        "{identity}/production-local-tangent/sequence={}/metres",
        diagnostic.sequence
    );
    value(
        snapshot,
        "conversion.path",
        format!("{:?}", diagnostic.frame.path),
    );
    value(snapshot, "conversion.game_s", step.game_s);
    value(snapshot, "conversion.elapsed_s", step.elapsed_s);
    vector(snapshot, "step.body_before", step.before.position, &cell);
    vector(snapshot, "step.body_after", step.after.position, &cell);
    let metres = 1.0 / universe_core::body::metre(universe.level());
    vector(
        snapshot,
        "step.gravity_m_per_s2",
        step.gravity_cell_per_s2.map(|v| v * metres),
        &format!("{identity}/metres-per-second-squared"),
    );
    value(
        snapshot,
        "step.strongest_gravity_m_per_s2",
        step.strongest_m_per_s2,
    );
    witness(
        snapshot,
        "conversion.input",
        diagnostic.frame.input,
        &cell,
        &local,
    );
    witness(
        snapshot,
        "conversion.output",
        diagnostic.frame.output,
        &cell,
        &local,
    );
    if let Some(anchor) = diagnostic.frame.anchor {
        vector(
            snapshot,
            "conversion.anchor.origin_m",
            anchor.origin.0,
            &format!("{identity}/planet-fixed-metres"),
        );
        let direction = cell.replace("open-cell-units", "direction");
        vector(snapshot, "conversion.anchor.east", anchor.east, &direction);
        vector(
            snapshot,
            "conversion.anchor.north",
            anchor.north,
            &direction,
        );
        vector(snapshot, "conversion.anchor.up", anchor.up, &direction);
    } else {
        unavailable(
            snapshot,
            "conversion.anchor.origin_m",
            "no-production-local-anchor-used",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_plugin_does_not_install_the_optional_diagnostic() {
        let app = crate::window_tests::headless_app();
        assert!(!app.world().contains_resource::<ProductionDiagnostic>());
        let mut diagnostic = ProductionDiagnostic::default();
        let body = Body::standing([0.0; 3], 0.0);
        diagnostic.finish(BodyStepRecord {
            seed: 42,
            open_seed: 0,
            elapsed_s: 0.0,
            game_s: 0.0,
            before: body,
            after: body,
            gravity_cell_per_s2: [0.0; 3],
            strongest_m_per_s2: 0.0,
        });
        assert_eq!(
            diagnostic.max_position_m, None,
            "missing conversion is not zero"
        );
        assert_eq!(diagnostic.max_direction_deg, None);
    }
}
