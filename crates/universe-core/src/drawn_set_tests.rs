use super::*;
use crate::nav::{AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode};

const DT: f64 = 1.0 / 60.0;

/// Walks the journey in to L7 and back out to L1 at 60 frames a second,
/// with the layer fades, and returns the largest change of the summed fade
/// between two frames at a handover and between two plain frames.
fn walk() -> (f64, f64, usize) {
    let mut universe = Universe::new(DEMO_SEED);
    let mut previews = PreviewCache::default();
    let mut fades = LayerFades::settled();
    let mut set = Vec::new();
    let factor = (-AUTOPILOT_RATE * DT).exp();
    let mut target = None;
    let (mut worst_handover, mut worst_plain, mut handovers) = (0.0f64, 0.0f64, 0);
    let mut inward = true;
    for _ in 0..40_000 {
        if inward && universe.level().get() >= 7 {
            inward = false;
        }
        if !inward && universe.path.is_at_root() {
            break;
        }
        previews.sync(&universe);
        drawn_set(&universe, &previews, &fades, &mut set);
        let before = shown(&set, universe.path.offset());
        let drawn_before = set.clone();
        let event = if inward {
            if target.is_none() {
                target = universe.journey_target();
            }
            universe.dive(target, factor, DiveMode::Journey)
        } else {
            universe.dive(None, 1.0 / factor, DiveMode::Passing)
        };
        let handover = match event {
            DiveEvent::Opened(_) => {
                target = None;
                if let Some(entered) = universe.path.entered() {
                    fades.on_open(&leaving_on_open(&drawn_before, entered), entered);
                }
                true
            }
            DiveEvent::Closed(opened) => {
                if inward {
                    target = Some(opened.marker);
                }
                fades.on_close();
                true
            }
            DiveEvent::Moved => false,
        };
        fades.tick(DT);
        previews.sync(&universe);
        drawn_set(&universe, &previews, &fades, &mut set);
        let after = shown(&set, universe.path.offset());
        let change = (after - before).abs();
        if handover {
            handovers += 1;
            worst_handover = worst_handover.max(change);
        } else {
            worst_plain = worst_plain.max(change);
        }
    }
    (worst_handover, worst_plain, handovers)
}

#[test]
fn nothing_pops_at_a_handover_in_or_out() {
    // #998 Spec v2 AC3: across every handover, in and out, the summed size
    // fade of what is drawn changes by no more than an ordinary frame of the
    // zoom does, plus one: the entered marker itself, which becomes the
    // cell's shell (drawn as the shell, not as a marker) at the open.
    let (handover, plain, count) = walk();
    println!("DRAWN handovers={count} worst_handover={handover:.2} worst_plain={plain:.2}");
    assert_eq!(count, 12, "six opens in, six closes out");
    assert!(plain <= 2.0, "a plain frame changed the picture by {plain:.2} markers");
    assert!(
        handover <= plain + 1.0,
        "a handover changed the picture by {handover:.2} markers (plain frames {plain:.2})"
    );
}

#[test]
fn the_fade_by_size_runs_from_half_a_pixel_to_two() {
    let px = pixel_rad();
    assert_eq!(size_fade(0.4 * px, 1.0), 0.0);
    assert!((size_fade(2.0 * px, 1.0) - 1.0).abs() < 1e-9);
    let mid = size_fade(1.25 * px, 1.0);
    assert!(mid > 0.4 && mid < 0.6, "{mid}");
}

#[test]
fn layer_fades_take_half_a_second() {
    let mut fades = LayerFades::settled();
    fades.on_close();
    assert_eq!(fades.grand, 0.0);
    for _ in 0..15 {
        fades.tick(DT);
    }
    assert!((fades.grand - 0.5).abs() < 1e-9);
    for _ in 0..15 {
        fades.tick(DT);
    }
    assert!((fades.grand - 1.0).abs() < 1e-9);
}
