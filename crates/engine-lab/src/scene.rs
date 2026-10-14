//! What the lab shows: seed, star path, planet, and the level on show
//! (#640 Q4b, #654 D5).
//!
//! The lab opens on the game's home star (the first eight markers of the
//! Spacebar journey) and its home planet. A seed field rebuilds the
//! universe, a path field (the L1-L9 chain, markers joined by `/`) jumps
//! to another star, and Previous/Next step through that star's planets.
//! A wrong seed or path never changes the planet: it leaves a message.
//! Parent and Child move the view through any level L1-L10 with the
//! game's own open and close (#654).
//! Pure and deterministic (`E-DET-TIERS`): the same seed and path always
//! give the same planet, through the game's own generation.

use crate::nodes::Node;
use universe_core::coords::Level;
use universe_core::r#gen::{Generated, MarkerKind, Point};
use universe_core::nav::{DEMO_SEED, MarkerIndex, Universe};
use universe_core::nest::{autopilot_path, generate_cell};

/// Markers from L1 to a star's close-up cell (L9).
pub const STAR_CHAIN: usize = 8;

/// The planet level: a full chain of markers from L1 opens it.
pub const PLANET_CHAIN: usize = STAR_CHAIN + 1;

/// The planet on show, the way to it, and the level on show.
#[derive(Debug, Clone, PartialEq)]
pub struct LabScene {
    seed: u64,
    star: Vec<u32>,
    planets: Vec<u32>,
    selected: usize,
    error: Option<String>,
    /// The chain of the cell on show (0 to 9 markers: L1 to L10, #654).
    view: Vec<u32>,
}

/// The star cell's planets: its portals carrying planet data, in marker
/// order.
fn planets_of(cell: &Generated) -> Vec<u32> {
    cell.points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal && point.planet.is_some())
        .filter_map(|(index, _)| u32::try_from(index).ok())
        .collect()
}

/// The star close-up at `star` under `seed`, or why it is not one: every
/// marker must open, in order, down to an L9 cell holding planets.
fn star_planets(seed: u64, star: &[u32]) -> Result<Vec<u32>, String> {
    if star.len() != STAR_CHAIN {
        return Err(format!(
            "a path has {STAR_CHAIN} markers from L1 to a star, not {}",
            star.len()
        ));
    }
    let mut universe = Universe::new(seed);
    for (depth, &marker) in star.iter().enumerate() {
        if !universe.open(MarkerIndex(marker)) {
            return Err(format!("marker {marker} does not open at L{}", depth + 1));
        }
    }
    let planets = planets_of(&universe.open);
    if planets.is_empty() {
        return Err("this path reaches no star with planets".to_string());
    }
    Ok(planets)
}

/// Parses a path like `7/2/7/1/2/0/0/0` (spaces allowed).
///
/// # Errors
///
/// A message naming the first part that is not a marker number.
pub fn parse_path(text: &str) -> Result<Vec<u32>, String> {
    text.split('/')
        .map(str::trim)
        .map(|part| {
            part.parse::<u32>()
                .map_err(|_| format!("\"{part}\" is not a marker number"))
        })
        .collect()
}

impl LabScene {
    /// The home star and home planet of `seed` (the game's start, Q4b).
    ///
    /// # Errors
    ///
    /// A message when the seed's journey reaches no star with planets.
    pub fn home(seed: u64) -> Result<LabScene, String> {
        let journey = autopilot_path(seed);
        let star: Vec<u32> = journey.iter().copied().take(STAR_CHAIN).collect();
        let planets = star_planets(seed, &star)?;
        let home = journey.get(STAR_CHAIN).copied();
        let selected = home
            .and_then(|marker| planets.iter().position(|&p| p == marker))
            .unwrap_or(0);
        let mut scene = LabScene {
            seed,
            star,
            planets,
            selected,
            error: None,
            view: Vec::new(),
        };
        scene.view = scene.planet_chain();
        Ok(scene)
    }

    /// The default scene: the game's demo seed (Earth with its Moon).
    #[must_use]
    pub fn default_home() -> LabScene {
        LabScene::home(DEMO_SEED).unwrap_or(LabScene {
            seed: DEMO_SEED,
            star: Vec::new(),
            planets: Vec::new(),
            selected: 0,
            error: Some("the demo seed reaches no planet".to_string()),
            view: Vec::new(),
        })
    }

    /// The universe seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The star's path (L1-L9 markers).
    #[must_use]
    pub fn star(&self) -> &[u32] {
        &self.star
    }

    /// The path as the path field shows it.
    #[must_use]
    pub fn path_text(&self) -> String {
        let parts: Vec<String> = self.star.iter().map(u32::to_string).collect();
        parts.join("/")
    }

    /// The message of the last refused seed or path, if any (shown red).
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Number of planets around the star.
    #[must_use]
    pub fn planet_count(&self) -> usize {
        self.planets.len()
    }

    /// The selected planet, 0-based.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The slider label, "Planet i of N".
    #[must_use]
    pub fn label(&self) -> String {
        format!("Planet {} of {}", self.selected + 1, self.planets.len())
    }

    /// Refuses an input with `message` (shown red); the planet stays.
    pub fn refuse(&mut self, message: String) {
        self.error = Some(message);
    }

    /// Selects planet `index` (0-based), clamped to the star's planets,
    /// and shows it.
    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.planets.len().saturating_sub(1));
        self.view = self.planet_chain();
    }

    /// The next planet, wrapping after the last, on show.
    pub fn next(&mut self) {
        if !self.planets.is_empty() {
            self.select((self.selected + 1) % self.planets.len());
        }
    }

    /// The previous planet, wrapping before the first, on show.
    pub fn previous(&mut self) {
        if !self.planets.is_empty() {
            self.select((self.selected + self.planets.len() - 1) % self.planets.len());
        }
    }

    /// Rebuilds the universe from `seed`, on that seed's home star and
    /// planet (AC3). On failure the scene keeps its planet and says why.
    pub fn set_seed(&mut self, seed: u64) {
        match LabScene::home(seed) {
            Ok(scene) => *self = scene,
            Err(message) => self.error = Some(message),
        }
    }

    /// Jumps to the star at the path `text`, on its first planet (AC4). A
    /// wrong path keeps the planet and leaves a message.
    pub fn set_path_text(&mut self, text: &str) {
        let star = match parse_path(text) {
            Ok(star) => star,
            Err(message) => {
                self.error = Some(message);
                return;
            }
        };
        match star_planets(self.seed, &star) {
            Ok(planets) => {
                self.star = star;
                self.planets = planets;
                self.selected = 0;
                self.error = None;
                self.view = self.planet_chain();
            }
            Err(message) => self.error = Some(message),
        }
    }

    /// The named chain from L1 to the selected planet (L10).
    #[must_use]
    pub fn planet_chain(&self) -> Vec<u32> {
        let mut chain = self.star.clone();
        chain.extend(self.planets.get(self.selected).copied());
        chain
    }

    /// The game's universe opened down to the cell on show.
    #[must_use]
    pub fn universe(&self) -> Universe {
        let mut universe = Universe::new(self.seed);
        for &marker in &self.view {
            if !universe.open(MarkerIndex(marker)) {
                break;
            }
        }
        universe
    }

    /// The chain of the cell on show (L1 has none).
    #[must_use]
    pub fn view(&self) -> &[u32] {
        &self.view
    }

    /// The level on show, L1-L10.
    #[must_use]
    pub fn level(&self) -> Level {
        Level::new(u8::try_from(self.view.len() + 1).unwrap_or(u8::MAX)).unwrap_or(Level::MAX)
    }

    /// Whether a planet is on show (L10): the planet's own views apply.
    #[must_use]
    pub fn on_planet(&self) -> bool {
        self.view.len() == PLANET_CHAIN
    }

    /// The status bar's level text: "L9 - Planetary systems, path 7/2/7".
    #[must_use]
    pub fn level_text(&self) -> String {
        let parts: Vec<String> = self.view.iter().map(u32::to_string).collect();
        let path = if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join("/")
        };
        format!(
            "L{} - {}, path {path}",
            self.level().get(),
            universe_core::labels::level_label(self.level())
        )
    }

    /// The size of the level on show, as the game's scale readout writes
    /// it; `None` on a planet, where the height reads instead (#654 D7).
    #[must_use]
    pub fn level_size(&self) -> Option<String> {
        (!self.on_planet()).then(|| universe_core::labels::scale_label(self.level()))
    }

    /// Parent (Page Up): one level up, as the game closes a cell.
    ///
    /// # Errors
    ///
    /// A message at L1, which has no level above.
    pub fn parent(&mut self) -> Result<(), String> {
        if self.view.pop().is_none() {
            return Err("L1 has no parent".to_string());
        }
        self.error = None;
        Ok(())
    }

    /// Child (Page Down): opens `marker` of the cell on show, as the game
    /// opens it. Reaching a planet makes it the planet on show (its star
    /// and Previous/Next follow).
    ///
    /// # Errors
    ///
    /// A message when the marker does not open (nothing opens below a
    /// planet, #632).
    pub fn child(&mut self, marker: u32) -> Result<(), String> {
        if self.on_planet() {
            return Err("nothing opens below the planet (L11-L14 are its shells)".to_string());
        }
        let mut universe = self.universe();
        if !universe.open(MarkerIndex(marker)) {
            return Err(format!(
                "object {} does not open at L{}",
                u64::from(marker) + 1,
                self.level().get()
            ));
        }
        self.view.push(marker);
        if self.on_planet() {
            let star: Vec<u32> = self.view.iter().copied().take(STAR_CHAIN).collect();
            let mut planets = planets_of(&generate_cell(self.seed, &star));
            if !planets.contains(&marker) {
                planets.push(marker);
            }
            self.selected = planets.iter().position(|&p| p == marker).unwrap_or(0);
            self.star = star;
            self.planets = planets;
        }
        self.error = None;
        Ok(())
    }

    /// The Nodes list for `universe` (this scene's universe): the planet
    /// and its moons on a planet, the level's own objects with the game's
    /// names above it (#654 D6), then Camera, Light, Grid and Axes.
    #[must_use]
    pub fn nodes(&self, universe: &Universe) -> Vec<(Node, String)> {
        let mut rows = Vec::new();
        if self.on_planet() {
            rows.push((Node::Planet, Node::Planet.label()));
            let count = moons(universe).len();
            rows.extend((0..count).map(|i| (Node::Moon(i), Node::Moon(i).label())));
        } else {
            let level = self.level();
            for (index, point) in universe.open.points.iter().enumerate() {
                let Ok(marker) = u32::try_from(index) else {
                    continue;
                };
                let name =
                    universe_core::names::marker_name(self.seed, &self.view, level, marker, point);
                rows.push((Node::Marker(marker), name));
            }
        }
        for node in [Node::Camera, Node::Light, Node::Grid, Node::Axes] {
            rows.push((node, node.label()));
        }
        rows
    }

    /// The selected planet's L9 marker (its name and orbit).
    #[must_use]
    pub fn planet_point(&self) -> Option<Point> {
        let marker = *self.planets.get(self.selected)?;
        generate_cell(self.seed, &self.star)
            .points
            .get(marker as usize)
            .copied()
    }

    /// The selected planet's name, as the game names it.
    #[must_use]
    pub fn planet_name(&self) -> String {
        let Some(marker) = self.planets.get(self.selected).copied() else {
            return "no planet".to_string();
        };
        let level = Level::new(9).unwrap_or(Level::MIN);
        self.planet_point().map_or_else(
            || format!("planet {}", marker + 1),
            |point| universe_core::names::marker_name(self.seed, &self.star, level, marker, &point),
        )
    }
}

/// The moons of an opened planet `universe`: their points, in marker order.
#[must_use]
pub fn moons(universe: &Universe) -> Vec<Point> {
    universe
        .open
        .points
        .iter()
        .filter(|point| point.moon.is_some())
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lab_opens_on_the_home_planet_with_its_moons() {
        // C3 / AC2: the game's home planet (Earth and its Moon on the demo
        // seed), opened through the game's own generation.
        let scene = LabScene::default_home();
        assert_eq!(scene.error(), None);
        assert_eq!(scene.seed(), DEMO_SEED);
        let journey = autopilot_path(DEMO_SEED);
        assert_eq!(scene.planet_chain(), journey, "the game's home planet");
        let universe = scene.universe();
        assert_eq!(universe.level().get(), 10, "the planet itself");
        let surface = universe
            .open
            .points
            .iter()
            .find_map(|point| point.surface)
            .expect("a planet record");
        assert_eq!(surface.radius_earth, 1.0, "Earth");
        assert_eq!(moons(&universe).len(), 1, "with its Moon");
        assert_eq!(scene.planet_name(), "Earth");
    }

    #[test]
    fn the_same_seed_and_path_give_the_same_planet() {
        // C4 / AC3.
        let mut scene = LabScene::default_home();
        let earth = scene.universe().snapshot();
        scene.set_seed(1234);
        assert_eq!(scene.error(), None);
        assert_eq!(scene.seed(), 1234);
        let other = scene.universe().snapshot();
        assert_ne!(other, earth, "another seed, another planet");
        let again = LabScene::home(1234).expect("seed 1234 reaches a planet");
        assert_eq!(again.universe().snapshot(), other, "same seed, same planet");
        scene.set_seed(DEMO_SEED);
        assert_eq!(
            scene.universe().snapshot(),
            earth,
            "back to the same planet"
        );
    }

    #[test]
    fn a_wrong_path_keeps_the_planet_and_says_why() {
        // C5 / AC4: the path starts on the home star; a wrong one keeps the
        // planet and explains; a valid other star jumps to its first planet.
        let mut scene = LabScene::default_home();
        let journey = autopilot_path(DEMO_SEED);
        assert_eq!(scene.star(), &journey[..STAR_CHAIN]);
        assert_eq!(parse_path(&scene.path_text()), Ok(scene.star().to_vec()));
        let before = scene.planet_chain();
        for wrong in ["7/x/7", "1/2/3", "7/2/7/1/2/0/0/999"] {
            scene.set_path_text(wrong);
            assert!(scene.error().is_some(), "{wrong} explains itself");
            assert_eq!(scene.planet_chain(), before, "{wrong} keeps the planet");
        }
        // Another star: a sibling system in the home stellar neighbourhood
        // (L6), entered down to its star the way the journey goes.
        let other = (0..32u32)
            .map(|system| {
                let mut path = journey[..STAR_CHAIN - 3].to_vec();
                path.extend([system, 0, 0]);
                path
            })
            .find(|path| path.as_slice() != scene.star() && star_planets(DEMO_SEED, path).is_ok())
            .expect("the home galaxy has another star with planets");
        scene.set_path_text(&parse_path_text(&other));
        assert_eq!(scene.error(), None);
        assert_eq!(scene.star(), other.as_slice());
        assert_eq!(scene.selected(), 0, "the star's first planet");
    }

    fn parse_path_text(star: &[u32]) -> String {
        star.iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("/")
    }

    #[test]
    fn previous_and_next_step_through_every_planet() {
        // C6 / AC5: every planet of the star, "Planet i of N", each with its
        // own moons.
        let mut scene = LabScene::default_home();
        let count = scene.planet_count();
        assert!(count >= 2, "the home star has planets: {count}");
        scene.select(0);
        let mut seen = Vec::new();
        for i in 0..count {
            assert_eq!(scene.label(), format!("Planet {} of {count}", i + 1));
            let universe = scene.universe();
            assert_eq!(universe.level().get(), 10, "planet {i} opens");
            let _ = moons(&universe);
            seen.push(scene.planet_chain());
            scene.next();
        }
        assert_eq!(scene.selected(), 0, "Next wraps after the last");
        seen.dedup();
        assert_eq!(seen.len(), count, "every planet once");
        scene.previous();
        assert_eq!(
            scene.selected(),
            count - 1,
            "Previous wraps before the first"
        );
    }

    #[test]
    fn parent_and_child_move_through_every_level() {
        // C6 / AC5 of #654: Parent climbs from the planet (L10) to L1 one
        // level at a time as the game closes cells; Child opens the way
        // back down to the same planet; L1 has no parent and nothing opens
        // below the planet.
        let mut scene = LabScene::default_home();
        let planet = scene.planet_chain();
        assert!(scene.on_planet());
        assert_eq!(scene.level().get(), 10);
        for want in (1..=9u8).rev() {
            scene.parent().expect("a level above");
            assert_eq!(scene.level().get(), want);
            assert_eq!(scene.universe().level().get(), want, "the game's own cell");
            assert!(!scene.on_planet());
            assert!(
                scene.level_text().starts_with(&format!("L{want} - ")),
                "{}",
                scene.level_text()
            );
        }
        assert!(scene.parent().is_err(), "L1 has no parent");
        assert_eq!(scene.view(), &[] as &[u32]);
        for &marker in &planet {
            scene.child(marker).expect("the way down opens");
        }
        assert!(scene.on_planet(), "back on the planet");
        assert_eq!(scene.planet_chain(), planet);
        assert_eq!(scene.planet_name(), "Earth");
        assert!(scene.child(0).is_err(), "nothing opens below the planet");
        // A marker that does not open says why and keeps the view.
        scene.parent().expect("the star system");
        let before = scene.view().to_vec();
        assert!(scene.child(u32::MAX).is_err());
        assert_eq!(scene.view(), before.as_slice());
    }

    #[test]
    fn every_level_lists_its_objects() {
        // C7 / AC6 of #654: above the planet the Nodes list holds the
        // level's own objects with the game's names; on the planet, the
        // planet and its moons; Camera, Light, Grid and Axes everywhere.
        let mut scene = LabScene::default_home();
        let tools = [Node::Camera, Node::Light, Node::Grid, Node::Axes];
        let universe = scene.universe();
        let rows = scene.nodes(&universe);
        assert_eq!(rows[0], (Node::Planet, "Planet".to_string()));
        assert_eq!(rows[1].0, Node::Moon(0), "Earth's Moon");
        assert_eq!(
            rows.iter().rev().take(4).map(|r| r.0).collect::<Vec<_>>(),
            {
                let mut t = tools.to_vec();
                t.reverse();
                t
            }
        );
        while scene.parent().is_ok() {
            let universe = scene.universe();
            let rows = scene.nodes(&universe);
            let markers: Vec<_> = rows
                .iter()
                .filter(|(node, _)| matches!(node, Node::Marker(_)))
                .collect();
            assert_eq!(
                markers.len(),
                universe.open.points.len(),
                "L{}",
                scene.level().get()
            );
            assert!(!markers.is_empty(), "L{} has objects", scene.level().get());
            for (node, name) in markers {
                let Node::Marker(marker) = *node else {
                    unreachable!("filtered");
                };
                let point = universe.open.points[marker as usize];
                assert_eq!(
                    name,
                    &universe_core::names::marker_name(
                        scene.seed(),
                        scene.view(),
                        scene.level(),
                        marker,
                        &point
                    )
                );
            }
            assert!(rows.iter().all(|(node, _)| *node != Node::Planet));
            assert_eq!(rows.len(), universe.open.points.len() + tools.len());
        }
    }

    #[test]
    fn planet_views_are_greyed_above_the_planet() {
        // C8 / AC7 of #654: the planet's debug views apply on the planet
        // only, and above it the stats read the level size, not a height.
        use crate::stats::{FrameTimes, SceneReadout, overlay_lines};
        let mut scene = LabScene::default_home();
        assert!(scene.on_planet(), "views on");
        assert_eq!(scene.level_size(), None, "the planet reads its height");
        scene.parent().expect("L9");
        assert!(!scene.on_planet(), "views greyed at L9");
        let size = scene.level_size().expect("a size above the planet");
        assert_eq!(size, universe_core::labels::scale_label(scene.level()));
        let readout = SceneReadout {
            triangles: 0,
            meshes: 0,
            position_m: [0.0; 3],
            height_m: f64::NAN,
            shell: scene.level().get(),
            size: scene.level_size(),
        };
        let lines = overlay_lines(true, &FrameTimes::default(), &readout);
        assert!(
            lines.iter().any(|l| *l == format!("size {size}")),
            "{lines:?}"
        );
        assert!(lines.iter().all(|l| !l.starts_with("height")), "{lines:?}");
        assert!(lines.last().is_some_and(|l| l.starts_with("level ")));
        while scene.parent().is_ok() {
            assert!(!scene.on_planet() && scene.level_size().is_some());
        }
    }
}
