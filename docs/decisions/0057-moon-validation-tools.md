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
Frozen-conversion diagnostics retain the actual before/after encoding of one pose,
not movement between steps. Ground queries and contact identities are separate.
Sky records expose current production body spheres, ground triangle, camera and
horizon-cull inputs; they do not claim rendered-pixel occlusion readback. UI text
is escaped and layout/sky/snapshot limits fail closed rather than omit records.

Explicit normal-profile test launches use the ordinary production loader/saver and
Quit path only in marked disposable profiles. The existing in-memory scenario mode
keeps its no-file-I/O meaning. A later launch must not replay setup to manufacture
successful persistence. The runner retains partial evidence on all exits and checks
real-profile fingerprints without publishing contents.
`--scenario-profile <file> <new-dir> <profile>` requires a v2 loaded start and
final `quit`. That action sends ordinary input to the existing Menu Quit and
confirmation controls; only their production exit yields `NORMAL-QUIT` evidence.
The first-use marker permits explicit preparation only on the initial launch;
restarts reject every placement action. The public runner excludes real-profile,
overlapping, linked/reparse and hardlinked targets; seeding is new-profile-only.
Disposable before/after bytes must agree with production serialization on Quit.

Collection is explicitly opt-in, bounded and streamed. Shared frame/time identities
link input, frame timing, readiness and resource witnesses. Per-frame/event records
retain transient high-water events and main/render-world retirement stages; logical
descriptors/bytes are not total device allocations or claims of physical GPU release.
Required dropped/missing records make a measurement incomplete. Runtime comparison
conditions and a main-agent build receipt must match the launched binary; no new
validator build permission or arbitrary command execution is granted.

`--loading` is explicit on either scenario launch; no environment variable installs
it. `loading-v1.tsv` streams scalar production worker/apply/inbox/cache/atlas and
main/render asset records through two reusable 8192-record buffers. The journal's
monotonic sequence/time and main-frame identities survive worker/body changes;
epochs retain their original surface owner, and proxies retain their actual marker.
The 8192-lifetime scalar registry owns no asset handles. Main store/entity/known-owner
changes, render-store residence/absence and prepared mesh/texture descriptors are
different stages. Shared grid/material reuse is distinct from outgoing proxies and
replaced atlases. Optional actual draw calls/GPU execution and physical GPU release
are unavailable, not zero. The sidecar has a 256 MiB limit. Overflow/poison/missing
required data is sticky non-success; sealing bounds the session, not an idle or
fully-retired shutdown claim. Integrated input timing/provenance/overhead remain
separate consumers of these loading records.

`--measure` on either scenario launch adds `input-measure-v1.tsv` and implies
loading collection. One opt-in `ObservationClock` supplies the main-frame and
monotonic identities for both journals and adjacent `MEASURE-LINK` trace records.
Intervals are First-to-First; main work, extracted-render work/waits and cap wait
remain separate records with their actual measured frame. A prior render sample
is never passed off as same-frame work. Selected per-frame snapshots retain their
actual observation frame, including the previous snapshot used by terminal input.
The 8192-record journal and 256 MiB sidecar fail closed on required loss. This
changes no existing route record or quality-controller sampling semantics.

Process samples use the runner's owned PID at one-second cadence. Bracketed UTC
captures on both sides link its separate monotonic clock to the app interval;
reports expose conservative frame ranges, capture uncertainty, drift checks and
sampling gaps, never exact simultaneous frame/process readings. A detected clock
discontinuity makes required correlation non-successful. Raw-derived summaries
include all preparation in session/resource peaks, separately label first-body
and repeated-body readiness (not disk-cache temperature), and exclude intervals
crossing scoring boundaries. Optional GPU readings remain unavailable.

`--timing-only` on either scenario launch retains only the common bounded
First-to-First/clock/actual-graphics-change journal. The same common records also
exist with `--measure`; collection-off installs no loading/full-state/work witness
or process-memory sampler. It is not a claim of completely uninstrumented timing.
Native read-only Windows condition snapshots are common to both sides and retain
owned-window/monitor mapping, exact nominal active-path refresh rational (plus the
separate GDI integer value), AC/battery/power scheme/overlay
and registry driver version. The actual app adapter must match the native driver.
Missing/unequal required scored conditions refuse a matched claim; one-second
snapshots cannot exclude intervening changes or unequal thermal/background load.

`scripts/build_release.py` is a fixed main-agent-only release builder outside
validator script permissions. Timed public launches require its receipt, verify
the clean source commit/tree and successful command against executable hash/size,
and retain the record. Read-only audits never execute its command field. Three
serial 30/120-second pairs compare median-on minus median-off intervals, retain
stalls and require growth within max(1 ms, 5%) mean / max(2 ms, 5%) p95. No validator
permission, build wrapper, ordinary startup behavior or quality choice changes.

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
