//! Bounded scenario lifecycle and read-only checks (#1227).

use crate::scenario_file::{Check, Scenario};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::time::Instant;

pub(crate) type Snapshot = BTreeMap<String, String>;

/// Opt-in run state. The app drains evidence every frame, including on failure.
#[derive(Resource, Debug)]
pub struct ScenarioRun {
    pub(crate) scenario: Scenario,
    pub(crate) launched: Instant,
    pub(crate) prepared: bool,
    pub(crate) started: bool,
    pub(crate) step: usize,
    pub(crate) entered: Instant,
    pub(crate) click_phase: u8,
    pub(crate) current: Snapshot,
    pub(crate) baselines: BTreeMap<String, Snapshot>,
    pub(crate) visited: Vec<String>,
    pub(crate) logged: bool,
    /// Streaming TSV lines. No file I/O in the render crate.
    pub pending: Vec<String>,
    /// None until every action finishes or a concrete failure occurs.
    pub outcome: Option<Result<(), String>>,
}

impl ScenarioRun {
    /// Starts the wall-clock bound before window initialization.
    #[must_use]
    pub fn new(scenario: Scenario) -> Self {
        let now = Instant::now();
        Self {
            scenario,
            launched: now,
            prepared: false,
            started: false,
            step: 0,
            entered: now,
            click_phase: 0,
            current: Snapshot::new(),
            baselines: BTreeMap::new(),
            visited: Vec::new(),
            logged: false,
            pending: Vec::new(),
            outcome: None,
        }
    }

    pub(crate) fn terminal(&mut self, result: Result<(), String>) {
        if self.outcome.is_none() {
            let line = match &result {
                Ok(()) => "SCENARIO-OK".into(),
                Err(s) => format!("SCENARIO-FAIL {s}"),
            };
            self.pending.push(line);
            self.outcome = Some(result);
        }
    }

    pub(crate) fn timed_out(&self, elapsed: f64) -> bool {
        elapsed >= self.scenario.timeout_s
    }

    pub(crate) fn advance(&mut self) {
        self.step += 1;
        self.entered = Instant::now();
        self.click_phase = 0;
        self.logged = false;
    }

    pub(crate) fn record(&mut self, name: &str) {
        self.pending.push(format!(
            "OBSERVE\t{name}\tstep={}\telapsed_s={:.6}",
            self.step,
            self.launched.elapsed().as_secs_f64()
        ));
        for (key, value) in &self.current {
            self.pending.push(format!("VALUE\t{name}\t{key}\t{value}"));
        }
    }
}

fn vector(value: &str) -> Option<Vec<f64>> {
    let values: Option<Vec<_>> = value
        .split(',')
        .map(|v| v.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect();
    values.filter(|v| !v.is_empty())
}

/// A failed or absent observation is a reason, never an implicit pass.
pub(crate) fn evaluate(
    check: &Check,
    now: &Snapshot,
    baselines: &BTreeMap<String, Snapshot>,
) -> Result<(), String> {
    let field = match check {
        Check::Equal(f, _) | Check::Range(f, ..) | Check::Present(f) | Check::Delta(f, ..) => f,
    };
    let value = now
        .get(field)
        .filter(|v| *v != "unavailable")
        .ok_or_else(|| format!("missing observation {field}"))?;
    let holds = match check {
        Check::Equal(_, want) => value == want,
        Check::Present(_) => true,
        Check::Range(_, lo, hi) => value
            .parse::<f64>()
            .is_ok_and(|v| v.is_finite() && v >= *lo && v <= *hi),
        Check::Delta(_, baseline, lo, hi) => {
            let old = baselines
                .get(baseline)
                .ok_or_else(|| format!("unknown baseline {baseline}"))?;
            // Every vector observation includes its reference context. Refuse
            // to subtract coordinates across a frame or units change.
            let frame = format!("{field}.frame");
            if now.get(&frame).is_none() || old.get(&frame) != now.get(&frame) {
                return Err(format!("delta frame mismatch for {field}"));
            }
            let a = old
                .get(field)
                .and_then(|v| vector(v))
                .ok_or("baseline is not a vector")?;
            let b = vector(value).ok_or("observation is not a vector")?;
            if a.len() != b.len() {
                return Err("delta vector lengths differ".into());
            }
            let distance = a
                .iter()
                .zip(b)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            distance.is_finite() && distance >= *lo && distance <= *hi
        }
    };
    if holds {
        Ok(())
    } else {
        Err(format!("unmet {check:?}; actual={value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_scenario;

    #[test]
    fn terminal_outcomes_are_once_only_and_incomplete_is_not_success() {
        let mut r = ScenarioRun::new(
            parse_scenario("scenario-v1\nseed 42\nsetup system\ntimeout 2\nobserve a").unwrap(),
        );
        assert_eq!(r.outcome, None);
        assert!(!r.timed_out(1.999));
        assert!(r.timed_out(2.0));
        r.terminal(Err("timeout".into()));
        r.terminal(Ok(()));
        assert_eq!(r.outcome, Some(Err("timeout".into())));
        assert_eq!(r.pending.len(), 1);
    }

    #[test]
    fn assertions_refuse_absent_nonfinite_and_mismatched_frames() {
        let mut now = Snapshot::from([
            ("p".into(), "1,2,3".into()),
            ("p.frame".into(), "planet-m".into()),
        ]);
        let old = BTreeMap::from([("start".into(), now.clone())]);
        assert!(
            evaluate(
                &Check::Delta("p".into(), "start".into(), 0., 0.),
                &now,
                &old
            )
            .is_ok()
        );
        now.insert("p.frame".into(), "another-planet-m".into());
        assert!(
            evaluate(
                &Check::Delta("p".into(), "start".into(), 0., 9.),
                &now,
                &old
            )
            .is_err()
        );
        assert!(evaluate(&Check::Present("absent".into()), &now, &old).is_err());
        now.insert("n".into(), "NaN".into());
        assert!(evaluate(&Check::Range("n".into(), 0., 1.), &now, &old).is_err());
    }
}
