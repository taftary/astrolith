use super::*;

const DT: f64 = 1.0 / 60.0;

#[test]
fn nothing_pops_at_a_handover_in_or_out() {
    // #998 Spec v2 AC3, AC5: across every handover, in and out, L7 into
    // the star-system space (L8) included, the summed size
    // fade of what is drawn changes by no more than an ordinary frame of the
    // zoom does, plus one: the entered marker itself, which becomes the
    // cell's shell (drawn as the shell, not as a marker) at the open.
    let walk = crate::headless_continuity::zoom_walk();
    let (handover, plain, count) = (walk.worst_handover, walk.worst_plain, walk.handovers);
    println!("DRAWN handovers={count} worst_handover={handover:.2} worst_plain={plain:.2}");
    assert_eq!(count, 14, "seven opens in (L1 to L8), seven closes out");
    assert!(
        plain <= 2.0,
        "a plain frame changed the picture by {plain:.2} markers"
    );
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

#[test]
fn marker_detail_sets_the_budget_and_is_saved() {
    // #998 S1 (AC6): Low / Normal / High draw 500 / 2,000 / 5,000 markers at
    // once, Normal by default, saved and reset with Display.
    use crate::settings::{Category, Settings};
    let budgets: Vec<usize> = MarkerDetail::ALL.iter().map(|d| d.budget()).collect();
    assert_eq!(budgets, [500, 2_000, 5_000]);
    let mut settings = Settings::defaults();
    assert_eq!(settings.marker_detail, MarkerDetail::Normal);
    settings.marker_detail = MarkerDetail::Low;
    let text = settings.format();
    assert!(text.contains("marker_detail = low"), "{text}");
    assert_eq!(Settings::parse(&text).marker_detail, MarkerDetail::Low);
    assert_eq!(
        settings.reset(Category::Display).marker_detail,
        MarkerDetail::Normal
    );
    assert_eq!(MarkerDetail::High.next(), MarkerDetail::Low);
}

#[test]
fn the_budget_keeps_the_largest_markers() {
    let near = |r: f64| Drawn {
        layer: Layer::Open,
        at: [0.0, 0.0, -1.0],
        radius: r,
        portal: false,
        weight: 1.0,
        inside: None,
    };
    let set = [near(0.01), near(0.02), near(0.03), near(0.04)];
    let mut angles = Vec::new();
    assert_eq!(
        budget_cutoff(&set, [0.0; 3], 4, &mut angles),
        0.0,
        "all fit"
    );
    let cut = budget_cutoff(&set, [0.0; 3], 2, &mut angles);
    // The two largest draw: the cutoff is the second largest's angle.
    assert!((cut - angular_radius(0.03, 1.0)).abs() < 1e-12, "{cut}");
}
