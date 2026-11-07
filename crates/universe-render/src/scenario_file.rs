//! Strict, versioned input scenarios (#1227, ADR 0056). No I/O or gameplay.

use crate::bindings::{Source, source};
pub(crate) use crate::scenario_fixture::Fixture;
use universe_core::keymap::Binding;

/// Maximum scenario text size and action count: traces remain bounded.
pub const SCENARIO_BYTES: usize = 131_072;
const MAX_STEPS: usize = 2_048;

/// A read-only assertion. Missing values never satisfy it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Check {
    Equal(String, String),
    Range(String, f64, f64),
    Present(String),
    Delta(String, String, f64, f64),
}

/// One action, processed through normal input or a read-only observation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Input(Source, bool),
    Pointer(f32, f32),
    Motion(f32, f32),
    Scroll(f32, f32),
    Click(String),
    Observe(String),
    Expect(Check),
    Wait(Check, f64),
    Hold(f64),
    /// Final public Menu > Quit > confirmation input, never a synthetic exit.
    Quit,
    /// Explicit v2 preparation, not measured input or travel.
    Prepare(String, Fixture),
}

/// Parsed opt-in scenario. Only the parser constructs validated scenarios.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub(crate) version: u8,
    pub(crate) seed: u64,
    pub(crate) fixture: Fixture,
    pub(crate) timeout_s: f64,
    pub(crate) steps: Vec<Step>,
}

fn number(text: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("not a finite number: {text}"))
}

fn seconds(text: &str) -> Result<f64, String> {
    let n = number(text)?;
    if n <= 0.0 || n > 1_800.0 {
        return Err("duration must be in (0, 1800] seconds".into());
    }
    Ok(n)
}

fn pair(a: &str, b: &str) -> Result<(f32, f32), String> {
    let (a, b) = (number(a)?, number(b)?);
    if a.abs() > 1_000_000.0 || b.abs() > 1_000_000.0 {
        return Err("pointer/scroll magnitude exceeds 1000000".into());
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: bounded input pixels narrowed for Bevy"
    )]
    Ok((a as f32, b as f32))
}

fn identifier(text: &str) -> Result<String, String> {
    if text.is_empty()
        || text.len() > 128
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(
            "expected an identifier (letters, digits, dot, dash, underscore; max 128)".into(),
        );
    }
    Ok(text.to_owned())
}

fn bounds(lo: &str, hi: &str) -> Result<(f64, f64), String> {
    let (lo, hi) = (number(lo)?, number(hi)?);
    if lo > hi {
        return Err("range minimum exceeds maximum".into());
    }
    Ok((lo, hi))
}

fn check(words: &[&str]) -> Result<Check, String> {
    match words {
        [field, "eq", value] => Ok(Check::Equal(identifier(field)?, identifier(value)?)),
        [field, "present"] => Ok(Check::Present(identifier(field)?)),
        [field, "range", lo, hi] => {
            let (lo, hi) = bounds(lo, hi)?;
            Ok(Check::Range(identifier(field)?, lo, hi))
        }
        [field, "delta", baseline, lo, hi] => {
            let (lo, hi) = bounds(lo, hi)?;
            if lo < 0.0 {
                return Err("delta range must be nonnegative".into());
            }
            Ok(Check::Delta(
                identifier(field)?,
                identifier(baseline)?,
                lo,
                hi,
            ))
        }
        _ => {
            Err("expected FIELD eq VALUE | present | range MIN MAX | delta BASELINE MIN MAX".into())
        }
    }
}

fn step(words: &[&str], version: u8, seed: u64) -> Result<Step, String> {
    match words {
        ["key", name, state] | ["button", name, state] => {
            let input = Binding::named(name)
                .and_then(source)
                .ok_or_else(|| format!("unknown input {name}"))?;
            if (words.first() == Some(&"key")) != matches!(input, Source::Keys(..)) {
                return Err("input kind does not match key/button command".into());
            }
            let down = match *state {
                "down" => true,
                "up" => false,
                _ => return Err("expected down or up".into()),
            };
            Ok(Step::Input(input, down))
        }
        ["pointer", x, y] => {
            let (x, y) = pair(x, y)?;
            Ok(Step::Pointer(x, y))
        }
        ["motion", x, y] => {
            let (x, y) = pair(x, y)?;
            Ok(Step::Motion(x, y))
        }
        ["scroll", x, y] => {
            let (x, y) = pair(x, y)?;
            Ok(Step::Scroll(x, y))
        }
        ["click", id] => Ok(Step::Click(identifier(id)?)),
        ["observe", id] => Ok(Step::Observe(identifier(id)?)),
        ["hold", time] => Ok(Step::Hold(seconds(time)?)),
        ["quit"] if version == 2 => Ok(Step::Quit),
        ["prepare", name, rest @ ..] if version == 2 => {
            let fixture = crate::scenario_fixture::parse(seed, rest)?;
            if fixture == Fixture::Loaded {
                return Err("loaded is an initial start, not a preparation action".into());
            }
            Ok(Step::Prepare(identifier(name)?, fixture))
        }
        ["expect", rest @ ..] => Ok(Step::Expect(check(rest)?)),
        ["wait", rest @ ..] => {
            let (time, rest) = rest.split_last().ok_or("wait needs a timeout")?;
            Ok(Step::Wait(check(rest)?, seconds(time)?))
        }
        _ => Err("unknown command or wrong argument count".into()),
    }
}

/// Parses `scenario-v1` or `scenario-v2`, seed, setup, timeout and bounded steps.
///
/// # Errors
/// Refuses unsupported versions, duplicate/out-of-order headers, malformed
/// commands, non-finite values and unbounded files/actions/durations.
pub fn parse_scenario(text: &str) -> Result<Scenario, String> {
    if text.len() > SCENARIO_BYTES {
        return Err("scenario exceeds 128 KiB".into());
    }
    let lines: Vec<_> = text
        .lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let line = line.split('#').next().unwrap_or("").trim();
            (!line.is_empty()).then_some((n + 1, line))
        })
        .collect();
    let header = |i: usize, key: &str| -> Result<&str, String> {
        let (n, line) = lines
            .get(i)
            .ok_or_else(|| format!("missing {key} header"))?;
        let words: Vec<_> = line.split_whitespace().collect();
        match words.as_slice() {
            [k, value] if *k == key => Ok(*value),
            _ => Err(format!("line {n}: expected {key} VALUE")),
        }
    };
    let version = match lines.first().map(|(_, l)| *l) {
        Some("scenario-v1") => 1,
        Some("scenario-v2") => 2,
        _ => return Err("expected scenario-v1 or scenario-v2".into()),
    };
    let seed = header(1, "seed")?
        .parse()
        .map_err(|_| "invalid seed (unsigned decimal)")?;
    let fixture = if version == 1 {
        crate::scenario_fixture::legacy(header(2, "setup")?)?
    } else {
        let (_, line) = lines.get(2).ok_or("missing setup header")?;
        let words: Vec<_> = line.split_whitespace().collect();
        let ["setup", rest @ ..] = words.as_slice() else {
            return Err("expected setup header".into());
        };
        crate::scenario_fixture::parse(seed, rest)?
    };
    let timeout_s = seconds(header(3, "timeout")?)?;
    let mut steps = Vec::new();
    for &(n, line) in lines.iter().skip(4) {
        steps.push(
            step(&line.split_whitespace().collect::<Vec<_>>(), version, seed)
                .map_err(|s| format!("line {n}: {s}"))?,
        );
        if steps.len() > MAX_STEPS {
            return Err("too many actions (maximum 2048)".into());
        }
    }
    if steps.is_empty() {
        return Err("scenario needs at least one action".into());
    }
    if steps
        .iter()
        .take(steps.len().saturating_sub(1))
        .any(|step| matches!(step, Step::Quit))
    {
        return Err("quit must be the final action".into());
    }
    let mut preparations = std::collections::BTreeSet::new();
    for step in &steps {
        if let Step::Prepare(name, _) = step
            && (!preparations.insert(name) || preparations.len() > 64)
        {
            return Err("preparation names must be unique, maximum 64".into());
        }
    }
    Ok(Scenario {
        version,
        seed,
        fixture,
        timeout_s,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "scenario-v1\nseed 42\nsetup planet\ntimeout 120\n";

    #[test]
    fn strict_headers_actions_and_limits() {
        let s = parse_scenario(&(HEADER.to_owned()+"key KeyW down\nhold 0.1\nkey KeyW up\nobserve start\nwait ready eq true 30\nexpect player.position delta start 0 10")).unwrap();
        assert_eq!(s.steps.len(), 6);
        for line in [
            "setup planet",
            "timeout 1",
            "key MouseLeft down",
            "button KeyW down",
            "key KeyW held",
            "pointer NaN 2",
            "motion inf 0",
            "hold 0",
            "hold 1801",
            "expect x range 2 1",
            "expect x delta y -1 2",
            "wait x eq true",
            "click",
            "observe a extra",
        ] {
            assert!(
                parse_scenario(&(HEADER.to_owned() + line)).is_err(),
                "{line}"
            );
        }
        assert!(parse_scenario(&HEADER.replace("scenario-v1", "scenario-v2")).is_err());
        assert!(
            parse_scenario(&(HEADER.to_owned() + &"observe x\n".repeat(MAX_STEPS + 1))).is_err()
        );
        assert!(parse_scenario(&"x".repeat(SCENARIO_BYTES + 1)).is_err());
    }

    #[test]
    fn checked_in_positive_and_negative_scenarios_have_explicit_contracts() {
        for fixture in [
            include_str!("../../../scripts/validation/scenarios/production-observations.scenario"),
            include_str!("../../../scripts/validation/scenarios/missing-sky-visibility.scenario"),
            include_str!("../../../scripts/validation/scenarios/selected-moon.scenario"),
            include_str!("../../../scripts/validation/scenarios/preparation-phases.scenario"),
            include_str!("../../../scripts/validation/scenarios/loaded-start.scenario"),
            include_str!("../../../scripts/validation/scenarios/missing-moon-ground.scenario"),
            include_str!("../../../scripts/validation/scenarios/planet.scenario"),
            include_str!("../../../scripts/validation/scenarios/planet-other.scenario"),
            include_str!("../../../scripts/validation/scenarios/near-moon.scenario"),
            include_str!("../../../scripts/validation/scenarios/system.scenario"),
            include_str!("../../../scripts/validation/scenarios/interstellar.scenario"),
            include_str!("../../../scripts/validation/scenarios/intergalactic.scenario"),
            include_str!("../../../scripts/validation/scenarios/menu.scenario"),
            include_str!("../../../scripts/validation/scenarios/scroll.scenario"),
            include_str!("../../../scripts/validation/scenarios/missing-control.scenario"),
            include_str!("../../../scripts/validation/scenarios/unmet.scenario"),
            include_str!("../../../scripts/validation/scenarios/timeout.scenario"),
        ] {
            parse_scenario(fixture).unwrap();
        }
        assert!(
            parse_scenario(include_str!(
                "../../../scripts/validation/scenarios/malformed.scenario"
            ))
            .is_err()
        );
        assert!(
            parse_scenario(include_str!(
                "../../../scripts/validation/scenarios/invalid-moon.scenario"
            ))
            .is_err()
        );
    }

    #[test]
    fn v2_headers_and_preparation_phases_are_explicit_bounded_and_unique() {
        let header = "scenario-v2\nseed 42\nsetup loaded\ntimeout 60\n";
        let prepare = "prepare visit moon 7.2.7.1.2.0.0.0.5 1 0 1 0 10 1000 normal 0 -10\n";
        let parsed = parse_scenario(
            &(header.to_owned() + "observe loaded\n" + prepare + "observe approached"),
        )
        .unwrap();
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.fixture, Fixture::Loaded);
        assert!(matches!(parsed.steps[1], Step::Prepare(..)));
        for bad in [
            header.to_owned() + prepare + prepare,
            header.to_owned() + "prepare x loaded\n",
            header.to_owned() + "setup loaded\n",
            header.replace("setup loaded", "setup planet") + "observe x\n",
            HEADER.to_owned() + prepare,
        ] {
            assert!(parse_scenario(&bad).is_err(), "{bad}");
        }
        let too_many = (0..65)
            .map(|n| prepare.replace("visit", &format!("visit{n}")))
            .collect::<String>();
        assert!(parse_scenario(&(header.to_owned() + &too_many)).is_err());
    }

    #[test]
    fn public_quit_is_explicit_v2_and_last_only() {
        let header = "scenario-v2\nseed 42\nsetup loaded\ntimeout 60\n";
        assert!(matches!(
            parse_scenario(&(header.to_owned() + "quit"))
                .unwrap()
                .steps
                .last(),
            Some(Step::Quit)
        ));
        assert!(parse_scenario(&(header.to_owned() + "quit\nobserve after")).is_err());
        assert!(parse_scenario(&(HEADER.to_owned() + "quit")).is_err());
    }

    #[test]
    fn persistence_fixtures_use_loaded_state_and_public_quit() {
        use crate::{ScenarioProfileStart, ScenarioRun};
        let save = include_str!("../../../scripts/validation/scenarios/profile-save.scenario");
        let run = ScenarioRun::new(parse_scenario(save).unwrap());
        assert!(run.for_profile(ScenarioProfileStart::Initial).is_ok());
        assert!(
            ScenarioRun::new(parse_scenario(save).unwrap())
                .for_profile(ScenarioProfileStart::Restart)
                .is_err()
        );
        for fixture in [
            include_str!("../../../scripts/validation/scenarios/profile-quit.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-recall.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-old-planet.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-old-space.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-missing-records.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-damaged-records.scenario"),
            include_str!(
                "../../../scripts/validation/scenarios/profile-unresolvable-records.scenario"
            ),
            include_str!("../../../scripts/validation/scenarios/profile-reset-player.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-reset-views.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-reset-display.scenario"),
            include_str!("../../../scripts/validation/scenarios/profile-incomplete-quit.scenario"),
        ] {
            assert!(
                ScenarioRun::new(parse_scenario(fixture).unwrap())
                    .for_profile(ScenarioProfileStart::Restart)
                    .is_ok()
            );
        }
        let no_quit =
            include_str!("../../../scripts/validation/scenarios/profile-no-quit.scenario");
        assert!(
            ScenarioRun::new(parse_scenario(no_quit).unwrap())
                .for_profile(ScenarioProfileStart::Initial)
                .is_err()
        );
    }
}
