//! Opt-in GPU pass timing for route runs (#1085 T1), never normal play.
//!
//! With `ASTROLITH_GPU_TIMERS=1` the route driver adds Bevy's render
//! diagnostics and this collector. Each new pass measurement is added to its
//! leg's scenario, so `gpu.txt` holds one mean per scenario and pass. These are
//! per-pass GPU execution times; passes overlap with CPU work and with each
//! other's waits, so they are never summed into a frame time. An adapter
//! without timestamp queries gives no `elapsed_gpu` values, and the file
//! then says `GPU-UNSUPPORTED` instead of reporting zeros.

use crate::RouteRun;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;

/// Per-scenario sums of GPU pass measurements (pure).
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct RouteGpu {
    /// `(scenario, pass path, sum, samples)` in first-seen order.
    rows: Vec<(String, String, f64, u32)>,
    /// Time of the last measurement taken per pass path.
    seen: Vec<(String, std::time::Instant)>,
}

/// Whether a diagnostic path is a pass time or count worth keeping (pure).
#[must_use]
pub(crate) fn wanted_pass(path: &str) -> bool {
    path.ends_with("elapsed_gpu")
        || path.ends_with("_invocations")
        || path.ends_with("clipper_primitives_out")
}

impl RouteGpu {
    /// Adds one measurement of `path` to `scenario`.
    pub fn record(&mut self, scenario: &str, path: &str, value: f64) {
        if !value.is_finite() || value < 0.0 {
            return;
        }
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|r| r.0 == scenario && r.1 == path)
        {
            row.2 += value;
            row.3 += 1;
        } else {
            self.rows
                .push((scenario.to_string(), path.to_string(), value, 1));
        }
    }

    /// Whether `path` has a measurement newer than the last one taken;
    /// remembers `time` when it does.
    fn is_new(&mut self, path: &str, time: std::time::Instant) -> bool {
        if let Some(last) = self.seen.iter_mut().find(|s| s.0 == path) {
            if last.1 >= time {
                return false;
            }
            last.1 = time;
        } else {
            self.seen.push((path.to_string(), time));
        }
        true
    }

    /// The `gpu.txt` lines: one `GPU-PASS` line per scenario and pass, or a
    /// single `GPU-UNSUPPORTED` line when no pass time was ever measured.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        if !self.rows.iter().any(|r| r.1.ends_with("elapsed_gpu")) {
            return vec!["GPU-UNSUPPORTED reason=no-elapsed-gpu-measurements".to_string()];
        }
        self.rows
            .iter()
            .map(|(scenario, path, sum, samples)| {
                format!(
                    "GPU-PASS scenario={scenario} samples={samples} mean={:.3} path={path}",
                    sum / f64::from(*samples)
                )
            })
            .collect()
    }
}

/// Installed only by the route driver when GPU timers are asked for.
#[derive(Debug, Clone, Copy, Default)]
pub struct RouteGpuPlugin;

impl Plugin for RouteGpuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RouteGpu>()
            .add_systems(PostUpdate, collect);
    }
}

/// Adds every new pass measurement to the current leg's scenario.
fn collect(store: Res<DiagnosticsStore>, run: Option<Res<RouteRun>>, mut gpu: ResMut<RouteGpu>) {
    let Some(run) = run else {
        return;
    };
    if run.done {
        return;
    }
    let Some(leg) = run.route.legs.get(run.leg) else {
        return;
    };
    for diagnostic in store.iter() {
        let path = diagnostic.path().as_str();
        if !wanted_pass(path) {
            continue;
        }
        let Some(m) = diagnostic.measurement() else {
            continue;
        };
        if gpu.is_new(path, m.time) {
            gpu.record(&leg.scenario, path, m.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn means_are_kept_per_scenario_and_pass() {
        let mut gpu = RouteGpu::default();
        gpu.record(
            "ground",
            "ground render/main_opaque_pass_3d/elapsed_gpu",
            12.0,
        );
        gpu.record(
            "ground",
            "ground render/main_opaque_pass_3d/elapsed_gpu",
            14.0,
        );
        gpu.record(
            "orbit",
            "ground render/main_opaque_pass_3d/elapsed_gpu",
            5.0,
        );
        assert_eq!(
            gpu.lines(),
            vec![
                "GPU-PASS scenario=ground samples=2 mean=13.000 path=ground render/main_opaque_pass_3d/elapsed_gpu",
                "GPU-PASS scenario=orbit samples=1 mean=5.000 path=ground render/main_opaque_pass_3d/elapsed_gpu",
            ]
        );
    }

    #[test]
    fn no_pass_time_is_reported_as_unsupported_not_zero() {
        let mut gpu = RouteGpu::default();
        assert_eq!(
            gpu.lines(),
            vec!["GPU-UNSUPPORTED reason=no-elapsed-gpu-measurements"]
        );
        gpu.record(
            "ground",
            "x/main_opaque_pass_3d/vertex_shader_invocations",
            10.0,
        );
        assert_eq!(
            gpu.lines(),
            vec!["GPU-UNSUPPORTED reason=no-elapsed-gpu-measurements"]
        );
    }

    #[test]
    fn bad_values_and_repeated_measurements_are_not_counted() {
        let mut gpu = RouteGpu::default();
        gpu.record("ground", "a/elapsed_gpu", f64::NAN);
        gpu.record("ground", "a/elapsed_gpu", -1.0);
        assert!(gpu.rows.is_empty());
        let t = std::time::Instant::now();
        assert!(gpu.is_new("a/elapsed_gpu", t));
        assert!(!gpu.is_new("a/elapsed_gpu", t));
        assert!(gpu.is_new("a/elapsed_gpu", t + std::time::Duration::from_millis(1)));
    }

    #[test]
    fn only_pass_times_and_counts_are_wanted() {
        assert!(wanted_pass("ground render/bloom/elapsed_gpu"));
        assert!(wanted_pass("ground render/ui/fragment_shader_invocations"));
        assert!(wanted_pass("ground render/ui/clipper_primitives_out"));
        assert!(!wanted_pass("ground render/ui/elapsed_cpu"));
        assert!(!wanted_pass("fps"));
    }
}
