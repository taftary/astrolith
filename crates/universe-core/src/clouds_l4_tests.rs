use super::*;
use crate::home::autopilot_path;
use crate::nav::DEMO_SEED;
use crate::nest::generate_cell;

#[test]
fn the_milky_way_holds_its_clouds_and_the_suns_region() {
    // #996 Goal 3: inside the Milky Way, its 8,107 clouds, and the journey
    // opens the region around the Sun, 8.2 kpc out, as marker 1 as before.
    let journey = autopilot_path(DEMO_SEED);
    assert_eq!(journey.get(3), Some(&1), "the journey keeps its L4 pick");
    let cell = generate_cell(DEMO_SEED, &journey[..3]);
    assert_eq!(cell.points.len(), MILKY_WAY_CLOUDS as usize);
    let sun = cell.points[1];
    assert_eq!(sun.kind, MarkerKind::Portal);
    let r = sun.position.iter().map(|c| c * c).sum::<f64>().sqrt();
    assert!((r - SUN_RADIUS_CELLS).abs() < 1e-12, "{r}");
    let open = cell
        .points
        .iter()
        .filter(|p| p.kind == MarkerKind::Portal)
        .count();
    assert!(open > 0 && open < cell.points.len() / 10, "{open}");
}

#[test]
fn other_galaxies_hold_clouds_by_shape_and_size() {
    let host = |kind, size| {
        Some(Host {
            galaxy: GalaxyInfo {
                kind,
                orientation: [0.0, 0.0, 1.0],
                barred: false,
                far_view: false,
            },
            size,
        })
    };
    let spiral = cloud_count(host(GalaxyKind::Spiral, 0.05), false);
    assert_eq!(spiral, MILKY_WAY_CLOUDS);
    assert!(cloud_count(host(GalaxyKind::Spiral, 0.1), false) > spiral);
    assert!(cloud_count(host(GalaxyKind::Irregular, 0.05), false) < spiral / 10);
    assert!(cloud_count(host(GalaxyKind::Elliptical, 0.05), false) < spiral / 10);
    assert_eq!(cloud_count(None, true), MILKY_WAY_CLOUDS);
}

#[test]
fn clouds_lie_in_the_host_plane() {
    let normal = [0.0, 0.0, 1.0];
    let host = Some(Host {
        galaxy: GalaxyInfo {
            kind: GalaxyKind::Spiral,
            orientation: normal,
            barred: true,
            far_view: false,
        },
        size: 0.05,
    });
    let mut points = vec![Point::bare([0.0; 3], 0.01, MarkerKind::Population); 400];
    lay_out(7, host, &mut points);
    let thin = points.iter().filter(|p| p.position[2].abs() < 0.03).count();
    assert!(thin > 300, "most clouds sit in the disk: {thin}");
    assert!(points.iter().all(|p| p.cloud.is_some()));
}
