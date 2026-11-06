//! Heights frozen before the water work (#1162 T1, Spec v2 criterion 7).
//!
//! `relief_golden_1162.txt` holds the height bits of the #1161 fixture
//! worlds (one of each type) at evenly spread directions, written from main
//! at f943143 before any water or lake change. Lakes may change heights only
//! inside their basins; every other height must keep these bits. Rewrite the
//! file only with `UPDATE_GOLDEN_1162=1` and only when the pull request says
//! why (the `E-GOLDEN` practice).

use crate::relief::Relief;
use crate::water_places::fibonacci_dir;

/// The fixture worlds by name and L10 chain (#1161 `surfaces.shots`).
const WORLDS: [(&str, [u32; 9]); 6] = [
    ("home", [7, 2, 7, 1, 2, 0, 0, 0, 5]),
    ("mercury", [7, 2, 7, 1, 2, 0, 0, 0, 0]),
    ("desert", [7, 2, 7, 2, 3781, 1, 0, 0, 0]),
    ("iron", [7, 2, 7, 9, 1294, 1, 0, 0, 1]),
    ("ice", [7, 2, 7, 1, 303, 1, 0, 0, 1]),
    ("lava", [7, 2, 7, 5, 4149, 1, 0, 0, 0]),
];

/// Directions per world.
const SAMPLES: usize = 1_500;

/// The frozen file.
const GOLDEN: &str = include_str!("relief_golden_1162.txt");

/// The file's text for today's heights.
fn render() -> String {
    let root = crate::nav::DEMO_SEED;
    let mut out =
        String::from("# #1162 T1 frozen heights: world, then one f64 bit pattern per direction\n");
    for (name, chain) in WORLDS {
        let relief = Relief::for_planet(root, &chain);
        out.push_str(name);
        for i in 0..SAMPLES {
            let h = relief.height_m(fibonacci_dir(i, SAMPLES));
            out.push_str(&format!(" {:016x}", h.to_bits()));
        }
        out.push('\n');
    }
    out
}

/// The frozen bits of world `name`.
fn frozen(name: &str) -> Vec<u64> {
    GOLDEN
        .lines()
        .find_map(|line| {
            let mut parts = line.split(' ');
            (parts.next() == Some(name)).then(|| {
                parts
                    .map(|p| u64::from_str_radix(p, 16).expect("hex bits"))
                    .collect()
            })
        })
        .expect("world in the golden file")
}

#[test]
fn heights_outside_lake_basins_keep_their_frozen_bits() {
    if std::env::var_os("UPDATE_GOLDEN_1162").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/relief_golden_1162.txt");
        std::fs::write(path, render()).expect("write golden");
        return;
    }
    let root = crate::nav::DEMO_SEED;
    for (name, chain) in WORLDS {
        let relief = Relief::for_planet(root, &chain);
        let bits = frozen(name);
        assert_eq!(bits.len(), SAMPLES, "{name}");
        for (i, then) in bits.into_iter().enumerate() {
            let dir = fibonacci_dir(i, SAMPLES);
            let now = relief.height_m(dir).to_bits();
            assert_eq!(now, then, "{name} direction {i} {dir:?}");
        }
    }
}
