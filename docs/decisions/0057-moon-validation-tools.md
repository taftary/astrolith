# 0057: Opt-in moon-validation support contracts

Status: accepted

Issue: #1238 (Spec v1; separate prerequisite for #1215).

## Context

The existing scenario-v1 input driver and camera routes cannot select arbitrary
moon approaches, inspect support/local-frame conversions, exercise production
save/restart or retain short-lived loading/resource events. Testing tools must
precede moon gameplay and must never fabricate its missing contact data.

## Decision

Extend the opt-in interfaces, not normal gameplay. Keep scenario-v1 and legacy
route/profiling semantics compatible. Version new scenario preparation explicitly:
selected existing identities, local directions/reference gaps, shared time and
look/mode; distinguish initial/multi-visit preparation from ordinary input. A
loaded-profile start observes production-loaded state before any setup. Repositioning
is never evidence of travel, and preparation remains in peak/resource accounting.

Core owns validated deterministic catalog/identity and geometric data, with no I/O
or renderer dependencies. `--list-moons [seed prefix planets moons]` prints a bounded
read-only catalog with `MOON` records and terminal `MOONS-OK`; nonexistent identities
produce `MOONS-FAIL` and exit 1, malformed CLI exits 2. A separate `MOON-BOUNDARY`
record labels the inclusive 200 km / exclusive 1700 km procedural domain, not a
sampled moon or a universal minimum. Object-kind evidence, instance origin,
property source and synthetic/procedural terrain remain separate.

Generic production diagnostics supply actual support, drawn-ground clearance,
gravity, local frames, frozen-pose conversions, sky/occlusion and UI/settings state.
Unavailable values carry reasons and never satisfy numeric/contact checks. A nearby
moon and a geometric reference sphere cannot stand in for supporting drawn ground.
Future moon gameplay populates the same contract through its production state.

Explicit normal-profile test launches use the ordinary production loader/saver and
Quit path only in marked disposable profiles. The existing in-memory scenario mode
keeps its no-file-I/O meaning. A later launch must not replay setup to manufacture
successful persistence. The runner retains partial evidence on all exits and checks
real-profile fingerprints without publishing contents.

Collection is explicitly opt-in, bounded and streamed. Shared frame/time identities
link input, frame timing, readiness and resource witnesses. Per-frame/event records
retain transient high-water events and main/render-world retirement stages; logical
descriptors/bytes are not total device allocations or claims of physical GPU release.
Required dropped/missing records make a measurement incomplete. Runtime comparison
conditions and a main-agent build receipt must match the launched binary; no new
validator build permission or arbitrary command execution is granted.

## Consequences

Normal startup installs neither a test driver nor a collector, even with profiling
environment variables set. Tests do not change game decisions, quality choices,
generation, save formats, Settings/Help, pins or resource budgets. Three serial
matched 30-second warm-up / 120-second scored off/on pairs must demonstrate the
approved collection-overhead limits before independent capability certification.

These are implementation contracts, not claims of already-proven capabilities.
Only independent real-application runs enter the capability registry. Moon landing,
walking, moon-relative saves, teleportation and owner appearance/feel acceptance
remain outside this prerequisite. Existing headless protocols/goldens are unchanged.
