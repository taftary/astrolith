# 0013 — Navigation model: pass-through dives and free flight

- Status: accepted
- Date: 2026-10-05
- Issue: #152 (Spec v1 approved; owner key decision recorded on the Issue)

## Context

Entry needed an explicit target, the camera moved on one radial line, and
free flight did not exist (realism review theme 5.6). The spec keeps the
fixed L1–L10 autopilot journey unchanged while adding two ways to move:
straight through whatever lies on the way, or free steering.

## Decision

Manual dives (`DiveMode::Passing`) open every crossed portal in along-ray
order (`crossed_portals`) while the stored target is only read, never
touched; crossed populations only preview. The autopilot
(`DiveMode::Targeted`, window and headless) opens only its stored target,
so the fixed seeded journey flies byte-identically to before. `F` toggles
target-dive vs free flight: entering keeps the camera and drops the target
(autopilot cancelled); leaving targets the nearest portal. Free flight
steers with WASD plus right-drag look at a surface-distance speed with
exponential steps: plain `1-8` recalls saves, `Shift+1-9` steps speed (the
spec claimed plain `1-8` for both; the owner resolved the overlap for
`Shift`). `Ctrl+1-8` saves camera plus target (or free pose),
memory only. Spacebar is dive-only: in free flight it returns to the dive
(nearest portal) first, then flies. The headless replay gains a scripted,
fixed-seed free-flight leg after the dive (300 steps, six-decimal
positions, rerun-identity and snapshot-match lines). Free flight opens and
closes nothing: markers are indicators, and content notions (#153+) revisit
crossing semantics. Clicking targets in dive mode only.

## Consequences

`E-STDOUT-PROTOCOL` gains the three `FREE-LEG` line formats
(`docs/engineering.md` lists them); golden `verify.txt` grows exactly those
three lines. The input system takes ten parameters (one per engine input),
allowed with the reason on the function. Window hover/click/fly/save steps
remain validator-unverifiable headless and are covered by verify, capture,
frame proof, and code reads instead.
