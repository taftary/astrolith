//! The developer prototype switch (#1085 T3): `ASTROLITH_PROTOTYPE=<name>`,
//! or a route file's `prototype <name>` line.
//!
//! T2 ranked VSync-on frame delivery first (#1085 T2 report), so the
//! prototypes change only *when* a frame reaches the screen, never what is
//! drawn, and never a player setting. A name is one or more parts joined by
//! `+`:
//!
//! - `latency<N>` (1 to 3): the window asks the graphics driver for at most
//!   N queued frames (Bevy's default is 2), which sets how many window images
//!   there are;
//! - `inflight<N>` (1 to 3): frame pacing waits once N frames are in flight
//!   (the #819 baseline is 2);
//! - `nopace`: no frame pacing wait at all; the window image alone holds
//!   the CPU back;
//! - `adaptive` (#1085 T6): two frames in flight with VSync off; with VSync
//!   on, one or two, whichever measures faster ([`crate::adaptive`]).
//!
//! `none` (or no switch) is the baseline. An unknown or repeated part is an
//! error, so a typo never runs as the baseline. The switch is never saved:
//! it is not a setting and never reaches `settings.toml`.

use bevy::prelude::Resource;
use std::num::NonZeroU32;

/// The environment variable that switches a prototype on.
pub(crate) const PROTOTYPE_ENV: &str = "ASTROLITH_PROTOTYPE";

/// Frames allowed in flight before pacing waits (#819 baseline).
pub(crate) const BASELINE_IN_FLIGHT: usize = 2;

/// Most frames a part may ask for.
const MOST_FRAMES: u32 = 3;

/// One chosen prototype, or the baseline (a resource in the window runs, so
/// their run line can name it).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prototype {
    /// The name as given, or `none`.
    pub name: String,
    /// The window's frame latency hint; `None` keeps Bevy's default.
    pub latency: Option<NonZeroU32>,
    /// Frames in flight before pacing waits; `None` turns pacing off.
    pub in_flight: Option<usize>,
    /// Whether the limit adapts between one and two (#1085 T6).
    pub adaptive: bool,
}

impl Default for Prototype {
    fn default() -> Self {
        Self {
            name: "none".to_string(),
            latency: None,
            in_flight: Some(BASELINE_IN_FLIGHT),
            adaptive: false,
        }
    }
}

/// Reads `latency<N>` / `inflight<N>` counts (pure): 1 to [`MOST_FRAMES`].
fn frames(part: &str, digits: &str) -> Result<NonZeroU32, String> {
    digits
        .parse::<u32>()
        .ok()
        .filter(|n| (1..=MOST_FRAMES).contains(n))
        .and_then(NonZeroU32::new)
        .ok_or_else(|| format!("prototype part '{part}' needs a count from 1 to {MOST_FRAMES}"))
}

impl Prototype {
    /// Reads a prototype name (pure).
    ///
    /// # Errors
    ///
    /// An unknown, repeated or out-of-range part, named in plain words.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() || text == "none" {
            return Ok(Self::default());
        }
        let mut chosen = Self {
            name: text.to_string(),
            ..Self::default()
        };
        let (mut latency_set, mut pacing_set) = (false, false);
        for part in text.split('+') {
            if let Some(digits) = part.strip_prefix("latency") {
                if latency_set {
                    return Err(format!("prototype '{text}' sets latency twice"));
                }
                chosen.latency = Some(frames(part, digits)?);
                latency_set = true;
            } else if let Some(digits) = part.strip_prefix("inflight") {
                if pacing_set {
                    return Err(format!("prototype '{text}' sets pacing twice"));
                }
                let n = frames(part, digits)?.get();
                chosen.in_flight = Some(usize::try_from(n).unwrap_or(BASELINE_IN_FLIGHT));
                pacing_set = true;
            } else if part == "adaptive" {
                if pacing_set {
                    return Err(format!("prototype '{text}' sets pacing twice"));
                }
                chosen.adaptive = true;
                pacing_set = true;
            } else if part == "nopace" {
                if pacing_set {
                    return Err(format!("prototype '{text}' sets pacing twice"));
                }
                chosen.in_flight = None;
                pacing_set = true;
            } else {
                return Err(format!(
                    "unknown prototype part '{part}' (known: latency1-3, inflight1-3, nopace, adaptive, none)"
                ));
            }
        }
        Ok(chosen)
    }

    /// The frame pacing this prototype asks for.
    pub(crate) fn pacing(&self) -> crate::gpu::FramePacingPlugin {
        crate::gpu::FramePacingPlugin {
            in_flight: self.in_flight,
            adaptive: self.adaptive,
        }
    }

    /// The switch from the environment, else the baseline.
    ///
    /// # Errors
    ///
    /// The variable holds a name [`Prototype::parse`] refuses.
    pub(crate) fn from_env() -> Result<Self, String> {
        std::env::var(PROTOTYPE_ENV).map_or_else(
            |_| Ok(Self::default()),
            |v| Self::parse(&v).map_err(|e| format!("{PROTOTYPE_ENV}: {e}")),
        )
    }

    /// The prototype a route run uses (pure): the route's line, else the
    /// environment's; both set and different is an error.
    ///
    /// # Errors
    ///
    /// Either name is refused, or the two disagree.
    pub(crate) fn for_route(route: Option<&str>, env: Option<&str>) -> Result<Self, String> {
        let from_route = route.map(Self::parse).transpose()?;
        let from_env = env
            .map(Self::parse)
            .transpose()
            .map_err(|e| format!("{PROTOTYPE_ENV}: {e}"))?;
        match (from_route, from_env) {
            (Some(r), Some(e)) if r != e => Err(format!(
                "route prototype '{}' and {PROTOTYPE_ENV}='{}' disagree",
                r.name, e.name
            )),
            (Some(r), _) => Ok(r),
            (None, Some(e)) => Ok(e),
            (None, None) => Ok(Self::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_switch_is_the_baseline() {
        let base = Prototype::default();
        assert_eq!(base.name, "none");
        assert_eq!(base.latency, None, "Bevy's own frame latency");
        assert_eq!(base.in_flight, Some(2), "the #819 two frames in flight");
        assert_eq!(Prototype::parse("").expect("empty"), base);
        assert_eq!(Prototype::parse(" none ").expect("none"), base);
        assert_eq!(Prototype::for_route(None, None).expect("unset"), base);
    }

    #[test]
    fn parts_combine() {
        let p = Prototype::parse("latency3+nopace").expect("valid");
        assert_eq!(p.name, "latency3+nopace");
        assert_eq!(p.latency, NonZeroU32::new(3));
        assert_eq!(p.in_flight, None);
        let p = Prototype::parse("inflight3").expect("valid");
        assert_eq!((p.latency, p.in_flight), (None, Some(3)));
        let p = Prototype::parse("adaptive").expect("valid");
        assert!(p.adaptive, "adaptive limit");
        assert_eq!(p.in_flight, Some(2), "starts at the baseline");
        assert!(p.pacing().adaptive);
        assert!(!Prototype::default().pacing().adaptive);
        let p = Prototype::parse("latency1").expect("valid");
        assert_eq!((p.latency, p.in_flight), (NonZeroU32::new(1), Some(2)));
    }

    #[test]
    fn a_typo_never_runs_as_the_baseline() {
        for wrong in [
            "latency",
            "latency0",
            "latency4",
            "inflight9",
            "nopace+inflight3",
            "latency2+latency3",
            "mailbox",
            "nopace+",
            "adaptive+inflight1",
            "adaptive+adaptive",
        ] {
            assert!(Prototype::parse(wrong).is_err(), "{wrong} accepted");
        }
    }

    #[test]
    fn a_route_and_the_environment_must_agree() {
        let r = Prototype::for_route(Some("nopace"), None).expect("route");
        assert_eq!(r.in_flight, None);
        let e = Prototype::for_route(None, Some("latency3")).expect("env");
        assert_eq!(e.latency, NonZeroU32::new(3));
        assert!(Prototype::for_route(Some("nopace"), Some("nopace")).is_ok());
        let err = Prototype::for_route(Some("nopace"), Some("latency3")).expect_err("clash");
        assert!(err.contains("disagree"), "{err}");
        let err = Prototype::for_route(None, Some("bogus")).expect_err("bad env");
        assert!(err.contains(PROTOTYPE_ENV), "{err}");
    }
}
