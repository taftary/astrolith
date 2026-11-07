//! Strict preparation-only fixture descriptions (#1238, ADR 0057).
//!
//! Selected identities are validated against unchanged generation, not resolved
//! by proximity or substituted at launch. Parsing never changes the game.

use universe_core::moon_approach::{
    FixtureLook, FixtureTimeSeconds, ReferenceGapMetres, SurfaceDirection,
};
use universe_core::moon_catalog::{MoonIdentity, PlanetIdentity};
use universe_core::nav::{MarkerIndex, Seed};

/// Versioned preparation context. Legacy v1 retains its exact fixed semantics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Fixture {
    Planet,
    OtherPlanet,
    NearMoon,
    System,
    Interstellar,
    Intergalactic,
    Selected(SelectedStart),
    /// Observe startup state without opening, placing, setting time or preferences.
    Loaded,
}

impl Fixture {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Planet => "planet",
            Self::OtherPlanet => "planet-other",
            Self::NearMoon => "near-moon",
            Self::System => "system",
            Self::Interstellar => "interstellar",
            Self::Intergalactic => "intergalactic",
            Self::Selected(start) => {
                if start.moon.is_some() {
                    "selected-moon"
                } else {
                    "selected-planet"
                }
            }
            Self::Loaded => "loaded",
        }
    }

    pub(crate) fn needs_ground(self) -> bool {
        match self {
            Self::Planet | Self::OtherPlanet => true,
            Self::Selected(start) => {
                start.moon.is_none()
                    && start.mode == StartMode::Normal
                    && universe_core::world_type::planet_type(
                        start.host.seed().0,
                        start.host.chain(),
                    )
                    .is_some()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartMode {
    Normal,
    Free,
    Dive,
}

/// All units/identity/direction invariants are checked during parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectedStart {
    pub host: PlanetIdentity,
    pub moon: Option<MoonIdentity>,
    pub direction: SurfaceDirection,
    pub gap: ReferenceGapMetres,
    pub time: FixtureTimeSeconds,
    pub mode: StartMode,
    pub look: FixtureLook,
}

pub(crate) fn legacy(name: &str) -> Result<Fixture, String> {
    match name {
        "planet" => Ok(Fixture::Planet),
        "planet-other" => Ok(Fixture::OtherPlanet),
        "near-moon" => Ok(Fixture::NearMoon),
        "system" => Ok(Fixture::System),
        "interstellar" => Ok(Fixture::Interstellar),
        "intergalactic" => Ok(Fixture::Intergalactic),
        _ => Err("unknown setup fixture".into()),
    }
}

fn finite(text: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("not a finite preparation number: {text}"))
}

fn selected(
    seed: u64,
    chain: &str,
    marker: Option<&str>,
    rest: &[&str],
) -> Result<Fixture, String> {
    let chain = chain
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "selected host needs a nine-index dot-separated planet chain")?;
    let host = PlanetIdentity::new(Seed(seed), &chain).map_err(|e| e.to_string())?;
    let moon = marker
        .map(|id| {
            let id = id
                .parse()
                .map_err(|_| "moon marker needs an unsigned decimal index")?;
            MoonIdentity::new(host, MarkerIndex(id)).map_err(|e| e.to_string())
        })
        .transpose()?;
    let [x, y, z, gap, time, mode, yaw, pitch] = rest else {
        return Err("selected start needs DX DY DZ GAP_M TIME_S MODE YAW_DEG PITCH_DEG".into());
    };
    let mode = match *mode {
        "normal" => StartMode::Normal,
        "free" => StartMode::Free,
        "dive" => StartMode::Dive,
        _ => return Err("selected mode needs normal, free or dive".into()),
    };
    let direction =
        SurfaceDirection::new([finite(x)?, finite(y)?, finite(z)?]).map_err(|e| e.to_string())?;
    let gap = ReferenceGapMetres::new(finite(gap)?).map_err(|e| e.to_string())?;
    let time = FixtureTimeSeconds::new(finite(time)?).map_err(|e| e.to_string())?;
    let look =
        FixtureLook::from_degrees(finite(yaw)?, finite(pitch)?).map_err(|e| e.to_string())?;
    Ok(Fixture::Selected(SelectedStart {
        host,
        moon,
        direction,
        gap,
        time,
        mode,
        look,
    }))
}

pub(crate) fn parse(seed: u64, words: &[&str]) -> Result<Fixture, String> {
    match words {
        ["loaded"] => Ok(Fixture::Loaded),
        ["planet", chain, rest @ ..] => selected(seed, chain, None, rest),
        ["moon", chain, marker, rest @ ..] => selected(seed, chain, Some(marker), rest),
        _ => Err("v2 setup needs loaded | planet HOST DX DY DZ GAP_M TIME_S MODE YAW_DEG PITCH_DEG | moon HOST ID DX DY DZ GAP_M TIME_S MODE YAW_DEG PITCH_DEG".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_contract_refuses_bad_identity_geometry_and_units() {
        let host = universe_core::nest::autopilot_path(42)
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".");
        let planet =
            PlanetIdentity::new(Seed(42), &universe_core::nest::autopilot_path(42)).unwrap();
        let id = universe_core::moon_catalog::moons_for_planet(planet).unwrap()[0]
            .identity
            .marker()
            .0
            .to_string();
        let text = format!("moon {host} {id} 1 1 0 10 86400 normal 10 -20");
        let Fixture::Selected(start) =
            parse(42, &text.split_whitespace().collect::<Vec<_>>()).unwrap()
        else {
            panic!("selected");
        };
        assert_eq!(start.host, planet);
        assert_eq!(start.gap.get(), 10.0);
        assert_eq!(start.time.get(), 86400.0);
        assert!(!Fixture::Selected(start).needs_ground());
        assert_eq!(Fixture::Selected(start).name(), "selected-moon");
        for bad in [
            text.replace(&format!("{host} {id}"), &format!("{host} 4294967295")),
            text.replace("1 1 0 10", "0 0 0 10"),
            text.replace(" 10 86400", " -1 86400"),
            text.replace("86400", "NaN"),
            text.replace("normal", "flying"),
            text.replace("-20", "91"),
        ] {
            assert!(
                parse(42, &bad.split_whitespace().collect::<Vec<_>>()).is_err(),
                "{bad}"
            );
        }
        assert_eq!(parse(42, &["loaded"]), Ok(Fixture::Loaded));
    }
}
