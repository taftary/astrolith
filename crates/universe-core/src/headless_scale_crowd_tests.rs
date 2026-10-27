use super::*;
use std::time::Instant;

#[test]
fn the_crowds_stay_inside_the_budget_along_the_journey() {
    // #998 AC3: live cells under the stream cap and the points of one
    // pixel or more under the starting budget, at every level L1-L7.
    let rows = crowds();
    let levels: Vec<u8> = rows.iter().map(|r| r.level).collect();
    assert_eq!(levels, [1, 2, 3, 4, 5, 6, 7]);
    for row in &rows {
        assert!(row.cells <= crate::stream::STREAM_CAP, "{row:?}");
        assert!(row.visible <= POINT_BUDGET, "{row:?}");
    }
}

#[test]
#[ignore = "study probe (#998): cargo test --release -p universe-core --lib scale_cost_prints -- --ignored --nocapture"]
fn scale_cost_prints() {
    // #998 AC4: the per-frame streaming and selection work, timed on one
    // thread (the phone-like run is the same work on one core), worst and
    // mean per level.
    for line in lines() {
        println!("{line}");
    }
    let mut walk = CrowdWalk::new();
    let mut per_level: Vec<(u8, f64, f64, usize)> = Vec::new();
    loop {
        let start = Instant::now();
        let (level, _, _) = walk.stream();
        let _ = walk.count();
        let ms = start.elapsed().as_secs_f64() * 1e3;
        match per_level.iter_mut().find(|r| r.0 == level) {
            Some(r) => {
                r.1 = r.1.max(ms);
                r.2 += ms;
                r.3 += 1;
            }
            None => per_level.push((level, ms, ms, 1)),
        }
        if !walk.step() {
            break;
        }
    }
    for (level, worst, total, n) in per_level {
        #[expect(clippy::cast_precision_loss, reason = "frame counts")]
        let mean = total / n as f64;
        println!("SCALE-COST level=L{level} mean_ms={mean:.3} worst_ms={worst:.3} frames={n}");
    }
}
