# Navigation

Scope: how the camera moves through the nested universe, from the
observable universe (L1) to a room (L14), and in particular how it should
behave from the planet (L10) through region, city, and building to the
room. The topic collects external research on multi-scale cameras, planet
approach, and level transitions, plus an analysis of Astrolith's current
camera against that research. It exists so that the specification
"Continuous camera from planet to room" can cite settled material instead
of restating it.

Overview: Astrolith is a continuous nested-universe viewer. Each level is
a cell of unit size; a child cell is a marker inside its parent at the
true size ratio; opening a marker is a pure origin shift and closing is
the exact inverse. The camera therefore never needs large coordinates,
but it does need a continuous pose, a sensible speed law, a defined up
vector near a planet, and gates that do not flicker. The owner reports
that navigation between levels feels worst from L10 down to L14. The
research documents here gather what globe viewers (CesiumJS, Google
Earth), space simulators (SpaceEngine, Gaia Sky, Celestia, Cosmographia),
planet engines (Outerra), games (Kerbal Space Program, No Man's Sky, Star
Citizen), and the zoomable-interface literature do about each of those
problems; the analysis maps the findings onto the current code.

## Documents

| Document | Type | What it covers |
|---|---|---|
| [Multi-scale camera](./multi-scale-camera.md) | research | Floating origin and relative-to-eye rendering, reverse-Z and logarithmic depth, speed proportional to distance, Powers of Ten pacing, pan-plus-zoom paths, survey of tools. |
| [Planet approach](./planet-approach.md) | research | Up-vector policies (north-up, local normal, constrained axis), CesiumJS altitude gates, fly-to and land, roll continuity and the look-at degeneracy, atmosphere entry in globes and games. |
| [Transitions and hysteresis](./transitions-and-hysteresis.md) | research and analysis | Schmitt-trigger gates, Astrolith's 0.02/0.10/0.14 rad thresholds, cross-fading by angular size, continuity invariants at a frame change, easing and turn-rate limits, zoomable user interfaces, terrain LOD. |
| [Analysis](./analysis.md) | analysis | The camera as implemented at commit `1364571`, a classified gap list (G1-G10), and candidate requirements R1-R12 for the specification. |
| [Sources](./sources.md) | sources | One record per external source (NAV-S001 to NAV-S030) with what each supports, plus the internal references read. |

Start with the analysis if you are writing the specification; start with
the research documents if you want the evidence.

## Images

| Image | Shows | Linked from |
|---|---|---|
| [Open and close hysteresis](../images/open-close-hysteresis.svg) | The preview, close, and open thresholds on the angular-radius axis with the shell and children brightness curves and the retained-state band. | [Transitions and hysteresis](./transitions-and-hysteresis.md#11-astroliths-gates-facts-from-the-ladder-and-code) |
| [Up-vector blend by altitude](../images/up-vector-blend-by-altitude.svg) | North-up far from the planet, local surface normal near the ground, and a blend band between, in planet radii. | [Planet approach](./planet-approach.md#1-the-up-vector-policy) |

Both images are author-drawn SVG diagrams (no external image was copied).

## Related topics

- Parent index: [Guides](../../README.md).
- [Universe](../../universe/documents/README.md): the content of each
  level; every level's `entry.md` narrates the dive into that level and
  quotes the same gates.
- [Realism review, section 2.3 Navigation and loading](../../universe/documents/realism-review.md#23-navigation-and-loading):
  the audit of the camera as it was at commit `dae3352`; sections
  [5.6](../../universe/documents/realism-review.md#56-loading-and-traversal-size-large)
  and [7](../../universe/documents/realism-review.md#7-loading-and-traversal-design-sketch)
  hold the earlier traversal sketch, now partly implemented (#403) and
  partly superseded by this topic.
- Frozen rules this topic reads but does not restate:
  `docs/universes/ladder.md` (R6 nesting and navigation, R7 preview
  fades, R9-R11 tail levels and milestones).

## Documentation status

Research complete for the questions listed in each document's scope;
gaps are named in each document's "Open questions" and in the sources'
limitations (Universe Sandbox, Elite Dangerous, and Outer Wilds have no
primary source yet; several academic papers were cited from metadata and
abstracts only). Nothing in this topic is a decision: values such as the
up-vector band edges, the open/close ratio, and per-level pacing are
left to the specification and the owner.
