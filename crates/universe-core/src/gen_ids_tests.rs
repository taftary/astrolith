//! Stable marker IDs on [`Generated`] (#1104 T2).

use super::*;

fn cell(n: usize, ids: Vec<u32>) -> Generated {
    Generated {
        points: (0..n)
            .map(|i| {
                let x = f64::from(u32::try_from(i).unwrap_or(0)) * 0.01;
                Point::bare([x, 0.0, 0.0], 0.001, MarkerKind::Portal)
            })
            .collect(),
        child_constraints: Vec::new(),
        ids,
    }
}

#[test]
fn an_identity_cell_numbers_markers_by_their_place() {
    let identity = cell(4, Vec::new());
    assert_eq!(identity.id_at(2), 2);
    assert_eq!(identity.index_of(3), Some(3));
    assert_eq!(identity.index_of(4), None, "past the end is not a marker");
    assert_eq!(
        identity.point_by_id(1).map(|p| p.position),
        Some([0.01, 0.0, 0.0])
    );
    let ids: Vec<u32> = identity.with_ids().map(|(id, _)| id).collect();
    assert_eq!(ids, vec![0, 1, 2, 3]);
}

#[test]
fn a_streamed_cell_numbers_markers_by_their_ids() {
    // Three resident markers out of a larger cell: IDs are not places.
    let streamed = cell(3, vec![4, 9, 70_001]);
    assert_eq!(streamed.id_at(2), 70_001);
    assert_eq!(streamed.index_of(9), Some(1));
    assert_eq!(streamed.index_of(1), None, "not resident");
    assert_eq!(
        streamed.point_by_id(70_001).map(|p| p.position),
        Some([0.02, 0.0, 0.0])
    );
    let ids: Vec<u32> = streamed.with_ids().map(|(id, _)| id).collect();
    assert_eq!(ids, vec![4, 9, 70_001]);
}

#[test]
fn every_generated_cell_on_the_journey_is_identity_numbered() {
    // Every journey cell keeps its resident markers at their places.
    let path = crate::home::autopilot_path(crate::nav::DEMO_SEED);
    for depth in 0..=path.len() {
        let chain = path.get(..depth).unwrap_or_default();
        let generated = crate::nest::generate_cell(crate::nav::DEMO_SEED, chain);
        // L1 numbers its fixture 0 to 7 explicitly (#1104 T4): still
        // its places.
        let identity = (0..generated.points.len()).all(|i| generated.id_at(i) as usize == i);
        assert!(identity, "depth {depth}");
    }
}
