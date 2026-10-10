//! Pick ranking, click cycling, and the label rule (#446).
//!
//! Pure input rules behind Spec v1 AC7 and AC12-AC14: portals first (the
//! portal on the way to the destination, then nearest), populations never
//! picked, repeated clicks cycling in the same order, and labels only on
//! portals that are large enough, selected, or hovered. Everything here is
//! allocation-free over caller-provided slices except the returned order,
//! which the window recomputes on click, not per frame.

/// Pick radius in logical pixels for hover and click (mouse).
pub const PICK_MOUSE_PX: f32 = 24.0;

/// Touch pick diameter in points, defined for when touch input arrives.
pub const PICK_TOUCH_PT: f32 = 44.0;

/// Touch pick diameter in density-independent pixels.
pub const PICK_TOUCH_DP: f32 = 48.0;

/// On-screen portal diameter in pixels at or above which it earns a label.
pub const LABEL_MIN_PX: f32 = 32.0;

/// Most labels drawn in one frame (largest first).
pub const LABEL_CAP: usize = 24;

/// Click distance within which a repeated click cycles instead of re-picking.
pub const CYCLE_RADIUS_PX: f32 = 6.0;

/// Most portals one cycle order holds (nearest wins past the cap).
pub const CYCLE_CAP: usize = 32;

/// Ranked pick order for portal candidates (Spec v1 AC13).
///
/// `candidates` holds `(marker, screen distance)` for portals only;
/// populations never reach this function. The portal on the way to the
/// destination wins first, then the nearest. Ties keep the smaller marker.
#[must_use]
pub fn pick_order(candidates: &[(u32, f32)], destination_marker: Option<u32>) -> Vec<u32> {
    let mut ranked: Vec<(u32, f32, bool)> = Vec::with_capacity(candidates.len());
    for &(marker, distance) in candidates {
        let on_way = destination_marker == Some(marker);
        ranked.push((marker, distance, on_way));
    }
    ranked.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked.into_iter().map(|(marker, _, _)| marker).collect()
}

/// Next portal on a repeated click in the same spot (Spec v1 AC14).
///
/// Returns the entry after `current` in `order`, wrapping to the head;
/// `None` selects the head. Empty orders yield `None`.
#[must_use]
pub fn cycle_next(order: &[u32], current: Option<u32>) -> Option<u32> {
    if order.is_empty() {
        return None;
    }
    let Some(held) = current else {
        return order.first().copied();
    };
    let mut after = false;
    for &marker in order {
        if after {
            return Some(marker);
        }
        if marker == held {
            after = true;
        }
    }
    order.first().copied()
}

/// Whether two clicks land on the same spot (a repeat, not a new pick).
#[must_use]
pub fn same_spot(first: [f32; 2], second: [f32; 2]) -> bool {
    let dx = first[0] - second[0];
    let dy = first[1] - second[1];
    dx.is_finite() && dy.is_finite() && (dx * dx + dy * dy).sqrt() <= CYCLE_RADIUS_PX
}

/// Resolves one click to a portal marker (Spec v1 AC13-AC14).
///
/// Empty orders pick nothing. A repeat click on the same spot cycles to the
/// next portal in the ranked order, wrapping; a click elsewhere (or the
/// first) takes the order head. `last` is the previous click position plus
/// the marker it picked, if any.
#[must_use]
pub fn resolve_click(
    order: &[u32],
    last: Option<([f32; 2], u32)>,
    cursor: [f32; 2],
) -> Option<u32> {
    let &[head, ..] = order else {
        return None;
    };
    let Some((pos, picked)) = last else {
        return Some(head);
    };
    if same_spot(pos, cursor) {
        cycle_next(order, Some(picked))
    } else {
        Some(head)
    }
}

/// Whether a portal earns a text label (Spec v1 AC7).
///
/// True when its on-screen diameter reaches [`LABEL_MIN_PX`], or it is the
/// selection, or it is under the pointer. Populations never qualify: callers
/// check the kind first.
#[must_use]
pub fn should_label(on_screen_diameter_px: f32, selected: bool, hovered: bool) -> bool {
    selected || hovered || (on_screen_diameter_px.is_finite() && on_screen_diameter_px >= LABEL_MIN_PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_portal_wins_then_nearest() {
        let candidates = [(1u32, 10.0f32), (2, 4.0), (3, 7.0)];
        assert_eq!(pick_order(&candidates, Some(3)), vec![3, 2, 1]);
        assert_eq!(pick_order(&candidates, None), vec![2, 3, 1]);
        assert_eq!(pick_order(&[], None), Vec::<u32>::new());
    }

    #[test]
    fn cycling_wraps_and_resets_on_a_move() {
        let order = [2u32, 3, 1];
        assert_eq!(cycle_next(&order, None), Some(2));
        assert_eq!(cycle_next(&order, Some(2)), Some(3));
        assert_eq!(cycle_next(&order, Some(1)), Some(2));
        assert_eq!(cycle_next(&[], Some(2)), None);
        assert!(same_spot([10.0, 10.0], [12.0, 11.0]));
        assert!(!same_spot([10.0, 10.0], [100.0, 100.0]));
        assert!(!same_spot([f32::NAN, 0.0], [0.0, 0.0]));
    }

    #[test]
    fn clicks_resolve_ranked_first_then_cycling() {
        let order = [2u32, 3, 1];
        assert_eq!(resolve_click(&[], None, [0.0, 0.0]), None);
        assert_eq!(resolve_click(&order, None, [0.0, 0.0]), Some(2));
        assert_eq!(
            resolve_click(&order, Some(([10.0, 10.0], 2)), [11.0, 10.0]),
            Some(3)
        );
        assert_eq!(
            resolve_click(&order, Some(([10.0, 10.0], 1)), [11.0, 10.0]),
            Some(2)
        );
        assert_eq!(
            resolve_click(&order, Some(([10.0, 10.0], 2)), [100.0, 100.0]),
            Some(2)
        );
    }

    #[test]
    fn labels_need_size_selection_or_hover() {
        assert!(should_label(LABEL_MIN_PX, false, false));
        assert!(should_label(0.0, true, false));
        assert!(should_label(0.0, false, true));
        assert!(!should_label(LABEL_MIN_PX - 1.0, false, false));
        assert!(!should_label(f32::NAN, false, false));
    }

    #[test]
    fn pick_tokens_match_the_spec_sizes() {
        assert_eq!(PICK_MOUSE_PX, 24.0);
        assert_eq!(PICK_TOUCH_PT, 44.0);
        assert_eq!(PICK_TOUCH_DP, 48.0);
    }
}
