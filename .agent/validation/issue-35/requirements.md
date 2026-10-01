# Requirements (DRAFT — UNCONFIRMED) — issue #35

## Title: Notion: Procedural universe game MVP (cosmic web to room)

## Owner source (issue body, verbatim):

# Resume

A game built around one core fantasy: a single, unbroken universe you can travel through, a survival game where they can do a lot of funny stuff we will define later. Now we focus on the mvp.

# Procedural Universe Spec: Cosmic Web → Room

**Status:** Draft v0.1
**Scale range:** 10²⁷ m → 10⁰ m (27 orders of magnitude)
**Levels:** 15 named levels (L1–L15)

## 1. Goals

- Generate a universe that can be traversed continuously from the cosmic web down to a 1 m room.
- Store nothing: all content is regenerated on demand from one root seed.
- Keep levels consistent: zooming in refines the parent, never contradicts it.
- Stay deterministic: the same seed and the same place always give the same result.

## 2. Scope

**In scope:**
Scale ladder, coordinate system, seeding, per-level generators, LOD and streaming, camera model.

**Out of scope (for now):**
Gameplay, physics simulation, multiplayer, sub-1 m detail, time evolution of the universe.

## 3. Notation

| Symbol | Meaning |
|---|---|
| `ℓ` | Level index, 1 (observable universe) to 15 (room) |
| `S_ℓ` | Characteristic size of level ℓ, in meters |
| `e_ℓ` | Order of magnitude, `e_ℓ = log₁₀(S_ℓ)` |
| `C = (ℓ, i, j, k)` | A cell: level plus integer grid index |
| `σ` | Seed (64-bit integer) |
| `σ₀` | Root seed of the universe |
| `H(·)` | Hash function (e.g. splitmix64 or xxHash) |
| `R(σ)` | Deterministic PRNG initialized from σ |
| `G_ℓ` | Generator function of level ℓ |
| `K_ℓ` | Constraints passed from level ℓ to level ℓ+1 |
| `F_ℓ` | Local coordinate frame of level ℓ |
| `o` | Local offset (float) inside the deepest active cell |
| `h` | Camera distance to the nearest significant surface or object |
| `θ` | Apparent angular size of an object, `θ ≈ S / d` |
| `θ_min` | Threshold above which an object is subdivided |

**Core formulas:**

- Seed derivation: `σ_child = H(σ_parent, ℓ, i, j, k)`
- Generator: `(contents_C, K_child) = G_ℓ( R(σ_C), K_parent )`
- Subdivide rule: subdivide `C` if `θ(C) > θ_min`
- Camera speed: `v = k_speed · h`

## 4. Scale ladder

| Level | Name | Typical size | Range e | Contents |
|---|---|---|---|---|
| L1 | Observable universe | ~10²⁷ m | 27 | Everything observable (~93 Gly across) |
| L2 | Cosmic web | 10²⁴–10²⁶ m | 24–26 | Filaments, walls, giant voids |
| L3 | Superclusters | 10²⁴–10²⁵ m | 24–25 | e.g. Laniakea (~5×10²⁴ m) |
| L4 | Galaxy clusters and groups | ~10²³ m | 23 | Virgo Cluster, Local Group |
| L5 | Galaxies | 10²⁰–10²¹ m | 20–21 | Milky Way (~10²¹ m) |
| L6 | Galactic structures | 10¹⁷–10¹⁹ m | 17–19 | Spiral arms, molecular clouds, star clusters |
| L7 | Stellar neighborhood | 10¹⁶–10¹⁷ m | 16–17 | Nearby stars (Alpha Centauri ~4×10¹⁶ m) |
| L8 | Outer solar system | 10¹⁵–10¹⁶ m | 15–16 | Oort cloud |
| L9 | Planetary system | 10¹²–10¹³ m | 12–13 | Planet orbits, Kuiper belt, heliopause |
| L10 | Stars | ~10⁹ m | 9 | The Sun (1.4×10⁹ m) |
| L11 | Planets and moons | 10⁶–10⁸ m | 6–8 | Jupiter, Earth (1.3×10⁷ m), the Moon |
| L12 | Regions of a planet | 10⁵–10⁶ m | 5–6 | Continents, countries, mountain ranges |
| L13 | Cities and landscapes | 10³–10⁵ m | 3–5 | Cities, lakes, valleys |
| L14 | Buildings | 10¹–10² m | 1–2 | Streets, houses, stadiums |
| L15 | Room | 10⁰–10¹ m | 0–1 | Furniture, layout, the 1 m end point |

Note: Some gaps between levels are large (10²¹ → 10¹⁹ m, 10⁹ → 10⁷ m). See §5.4 for intermediate cells.

## 5. Architecture

### 5.1 Coordinates and precision

A 64-bit float has about 15–16 significant digits, so one global float coordinate cannot span 10²⁷ m down to 1 m.

Solution: nested frames. A position is a chain of integer cell indices, one per active level, plus a small float offset in the deepest cell.

`P = [ (i₁,j₁,k₁), (i₂,j₂,k₂), ..., (iₘ,jₘ,kₘ), o ]`

Re-anchoring: when `|o|` exceeds the current cell bounds, the origin moves to the neighboring cell (or to the parent/child cell) and `o` is recomputed.

Precision budget: keep the ratio between frame size and required detail under ~10⁶ for float32 and ~10¹⁵ for float64.

### 5.2 Seeding and determinism

One root seed `σ₀` defines the whole universe.

Every cell derives its seed from its parent: `σ_child = H(σ_parent, ℓ, i, j, k)`.

Generators are pure functions: no global state, no dependence on visit order.

Same `(σ₀, ℓ, i, j, k)` always regenerates the identical cell.

### 5.3 Top-down constraint passing

Generation runs from L1 to L15. Each level hands constraints to the next:

- L2 filaments → galaxy placement density
- L5 galaxy → star density profile, spiral arm geometry
- L10 star → luminosity, mass, habitable zone
- L11 planet → radius, gravity, climate, terrain seed
- L13 city → road graph, building footprints

Rule: a child may add detail but must respect every constraint in `K_parent`.

### 5.4 Intermediate spatial cells

A galaxy has ~10¹¹ stars, so named levels are not enough. Between L5 and L10 use an octree (or grid) of anonymous cells:

- Depth `d` is chosen so that each leaf holds a manageable number of objects (e.g. ≤ 10³).
- Each octree node gets its own seed via `H`, exactly like a named level.
- These nodes are not shown in the ladder but appear in the cell path of `P`.

### 5.5 LOD and streaming

Only cells near the camera are generated, at the detail the camera can perceive.

Subdivide when `θ(C) > θ_min`; merge back into a coarse impostor when it falls below.

Cache generated cells in an LRU cache keyed by `(ℓ, i, j, k)`.

Generate in background threads and swap a coarse impostor for detail when ready.

## 6. Generator spec per level

| Level | Method | Input from parent | Output to child |
|---|---|---|---|
| L1–L3 | Density field (Voronoi foam or noise); point process on filaments | Root seed | Cluster positions, void map |
| L4–L5 | Statistical sampling of galaxy type, size, orientation | Local density | Galaxy shape, mass, star count |
| L6–L7 | Density-wave spiral arms, radial profile, star mass function | Galaxy profile | Star positions, masses, types |
| L8–L9 | Rule-based orbital spacing; planet types by distance from star | Star mass, luminosity | Orbits, planet parameters |
| L10 | Star model from mass and age | Stellar parameters | Radius, temperature, spectrum |
| L11 | Fractal noise on a cube-sphere quadtree; plate and erosion model | Planet parameters | Heightmap, biomes, climate |
| L12–L13 | Terrain features, settlement placement, road networks | Heightmap, biomes | City sites, road graph |
| L14 | Lot subdivision, shape grammars | Road graph, city style | Building footprints and shells |
| L15 | Constraint solving or wave function collapse | Building shell | Room layout, furniture |

Pattern: large scales are statistical (densities and distributions); small scales are structural (rules and grammars).

## 7. Rendering and camera

- Camera speed scales with altitude: `v = k_speed · h`, so travel feels constant at every level.
- Depth: logarithmic depth buffer, or layered rendering (far layer → near layer, each with its own frustum).
- Origin shifting: the render origin follows the camera to avoid jitter.
- Impostors: distant objects are billboards or point sprites until `θ > θ_min`.
- Level detection: the active level is derived from `log₁₀(h)`.

## 8. Invariants (must always hold)

- [ ] Determinism: same seed and place always produce the same content.
- [ ] Consistency: a child never contradicts `K_parent`.
- [ ] Order independence: results do not depend on which cells were generated first.
- [ ] Seamlessness: neighboring cells at the same level agree along shared borders.
- [ ] Bounded memory: the live cell count is limited by the LOD threshold, not by universe size.

## 9. Milestones

- [ ] M1: Ladder and notation. Freeze the 15 levels and symbol table (this document).
- [ ] M2: Core framework. Nested coordinates, seeding, cell cache, re-anchoring.
- [ ] M3: Top three levels. L1–L3 density field and cluster placement, visible as points.
- [ ] M4: Galaxy to star. L4–L10 with the octree between L5 and L10.
- [ ] M5: Planets. L11 cube-sphere terrain with LOD.
- [ ] M6: Surface scales. L12–L14 regions, cities, buildings.
- [ ] M7: Room. L15 interior generation, reaching the 1 m end point.
- [ ] M8: Full zoom test. Continuous zoom L1 → L15 with no visible seams or jitter.

## 10. Open decisions

| Question | Options | Decision |
|---|---|---|
| End use | Zoom-through visualization / explorable game / scientific simulation | TBD |
| Ladder granularity | 15 named levels / 27 clean power-of-ten steps | TBD |
| Realism | Plausible-looking / physically calibrated | TBD |
| Engine | Custom / Unity / Unreal / Godot / web (WebGL, WebGPU) | TBD |
| Numeric type | float64 + integer cells / fixed-point | TBD |
| Extra column | Add light-travel time to the ladder | TBD |

---
_Draft path: drafts/universe-mvp.md_
_Type: notion_
_Formatting note: body reformatted into Markdown tables/lists on owner request; no words changed from the original submission._


## Comments (verbatim):

---

## Clarification questions

Your notion is taken as Issue #35. Before I write the specification, I need your answers (your spec already lists most as TBD):

1. **Engine:** Custom / Unity / Unreal / Godot / web (WebGL, WebGPU)?
2. **Repo home:** implement in this repo (`astrolith`) or another repository (which one)?
3. **First scope:** the full M1-M8 ladder, or a first slice (e.g. M1 freeze ladder + M2 core framework)? If a slice, which milestones?
4. **End use:** zoom-through visualization / explorable game / scientific simulation?
5. **Realism:** plausible-looking / physically calibrated?
6. **Ladder granularity:** 15 named levels / 27 clean power-of-ten steps?
7. **Numeric type:** float64 + integer cells / fixed-point?
8. **Light-travel time:** add the extra column to the ladder / skip?

Reply with your answers (e.g. `1. web, 2. this repo, ...`) and I will write the specification.


---

## Owner decisions (from PR #36, folded in)

 surviving content of `drafts/tech-notion.md`, recorded here as decisions. PR #36 closed unmerged (drafts arrive via intake, not via PR).

**Decided:**
- Q1 Engine: custom � Rust stable (edition 2024, pinned toolchain), Bevy 0.19 (exact version pinned, deliberate upgrades, only used features enabled, deps checked against pinned Bevy).
- Targets: Windows, macOS, Linux, Android, iOS, consoles (list TBD). Priorities: performance, full control, AI-driven development.
- Architecture: Cargo workspace (core logic, rendering, app entry); core headless-testable; one plugin per feature; data-driven content; fixed timestep.
- Platforms: desktop-first dev/test until MVP; real-device Android/iOS testing from MVP; suspend/resume, touch, safe-area from start; iOS needs macOS+Xcode; consoles = custom ports (NDA SDKs, dev kits).
- Performance: profile desktop then weakest phone; no per-frame allocs in hot systems; change detection; KTX2/Basis; budgets before content; LTO + single codegen unit release; fast linker, dev-only dynamic linking.
- Tooling: Tracy, Android Studio profiler, Instruments; rustfmt, clippy, nextest; CI builds desktop + Android + iOS per commit; `CLAUDE.md` with pins/rules; read crate source, don`t guess Bevy APIs; every change passes check + clippy; headless tests per feature.

**Still open (carried into the question list):** 2D vs 3D; single/multiplayer + networking crate; physics crate; input crate; which consoles + who gets SDK access; minimum device spec; target frame rate. Plus from before: repo home, first scope slice, end use, realism, ladder granularity, numeric type, light-travel time.

**Known tension for spec stage:** Bevy transforms are f32; spec section 5.1 requires nested integer-cell frames past float range. Needs a custom coordinate layer above Bevy transforms.


---

## Owner answers (recorded 2026-10-01)

- Repo home: this repo (`astrolith`).
- First scope: M1+M2 slice � freeze the ladder + symbol table (M1), core framework with nested coordinates, seeding, cell cache, re-anchoring (M2).
- End use: explorable game.
- Realism: plausible-looking.
- Ladder granularity: 15 named levels (octree gap-fillers per 5.4).
- Numeric type: float64 + integer cells, custom layer above Bevy transforms.
- Light-travel column: skip for MVP.
- 3D. Single-player (no networking in MVP).
- Physics crate: defer to spec. Input crate: defer to spec.
- Consoles: deferred past MVP. Minimum device spec: defer to spec.
- Target frame rate: 60 fps.


---

## Specification (owner review)

**Goal:** In this repo, a Rust + Bevy starting point plus the two first milestones of your universe: the frozen 15-level ladder with its symbol table (M1), and the core framework that can name any place, regenerate it identically every time, remember nearby places, and move the origin without jumps (M2). 3D, one player, plausible-looking, 60 fps target.

**Non-goals:** No galaxies, stars, planets, cities, or rooms yet (M3-M8). No multiplayer. No consoles. No physics or input crate choice (deferred to the next spec). No gameplay.

**Acceptance criteria:**
1. The 15 levels and every symbol from your notion are written down once, in one document, and nothing contradicts them.
2. Any place in the universe can be named (level + cell numbers + small offset) and regenerated from the root seed with the identical result every time.
3. Nearby generated places are kept in a limited memory store; far ones are dropped.
4. Moving across a cell border recomputes the origin with no jump.
5. Automated checks prove 2-4 and run green.

**Test plan (what happens):**
1. I run the automated checks in front of you (they print pass/fail). Expected: all green.
2. I regenerate the same place twice from the seed and show identical output. Expected: byte-identical.
3. You open the frozen ladder document. Expected: 15 levels, every symbol explained, matching your notion.
4. You confirm the frame-rate and plausibility targets are written as budgets, not yet proven (that comes with M3+).

**Testability needs:** Nothing from you. Checks run headless on desktop; no accounts, no devices, no services.

---
Please reply `approved` on this Issue if this matches your notion, or tell me what to change.


---

## Specification v2 (owner review, supersedes v1 per your feedback)

**Goal:** Everything in v1, plus a visible result: a Bevy window where you move between levels of the ladder and see each level as minimal grids and vectors (indicators only, no meshes). Underneath, the tech stack is pinned down precisely (Cargo workspaces, features) and the code follows SOLID/DRY rules so it stays maintainable, clear, and documented.

**Non-goals:** No M3+ content (no galaxies, planets, cities). No meshes or textures. No gameplay. No multiplayer, consoles, physics/input crate choice.

**Acceptance criteria:**
1. The frozen 15-level ladder + symbol table in one document, matching your notion.
2. A Bevy window opens on desktop; you switch levels across the ladder and each level shows a minimal grid plus vector indicators (axes/scale). No meshes anywhere.
3. Same place regenerated twice gives identical output (determinism shown in automated checks).
4. Cargo workspace with named crates (core logic, rendering, app entry) and explicit feature flags; each dependency pinned and justified in one document.
5. Architecture rules written down and followed: core has no rendering dependency (runs headless), one plugin per feature, no duplicated logic (DRY), each piece has one job (SOLID), public items documented.

**Test plan:**
1. I run the automated checks. Expected: all green.
2. You open the window, switch levels up and down. Expected: grid + vectors change per level, no meshes, no crash.
3. You regenerate the same place twice (a button or key). Expected: identical indicators.
4. You open the architecture/stack document. Expected: crates, features, pins, and rules all listed.

**Testability needs:** Nothing from you. Desktop window + headless checks; no accounts, devices, or services.

---
Please reply `approved` on this Issue if v2 matches your notion, or tell me what to change.


---

## Specification v3 (owner review, supersedes v2)

Change from v2: scope extended to reach the planetary surface (M1-M5). Room, cities, and buildings stay out.

**Goal:** Travel from the cosmic web to a planet surface in a Bevy window: frozen 15-level ladder (M1), core framework with deterministic regeneration (M2), top three levels as visible points (M3), galaxies down to stars with the L5-L10 octree (M4), and planet terrain with heightmap LOD (M5). Every level shows indicators only (grids, vectors, points) � no meshes. Stack pinned precisely (Cargo workspaces, features) and SOLID/DRY architecture, documented.

**Non-goals:** No cities, buildings, rooms (M6-M8). No meshes/textures. No gameplay, multiplayer, consoles, physics/input crate choice.

**Acceptance criteria:**
1. Frozen ladder + symbol table in one document, matching your notion.
2. Window opens; you travel the chain cosmic web -> galaxies -> stars -> planet surface; each level shows indicators (grids, vectors, points), never meshes.
3. Same place twice gives identical output; neighbors agree at borders (spot-checked in automated checks).
4. Cargo workspace (core, rendering, app), explicit features, pinned deps, architecture rules (headless core, one plugin per feature, DRY, SOLID, documented) � all in one document, all followed.
5. Automated checks green, including determinism and border-agreement checks.

**Test plan:**
1. I run the automated checks. Expected: all green.
2. You fly the chain top to surface in the window. Expected: indicators at every level, no crash, no meshes.
3. You regenerate a place twice. Expected: identical.
4. You open the stack/architecture document. Expected: crates, features, pins, rules listed.

**Testability needs:** Nothing from you. Desktop window + headless checks.

---
Please reply `approved` on this Issue if v3 matches your notion, or tell me what to change.


---

## Specification v4 (owner review, supersedes v3)

Change from v3: testability section only. Everything else (M1-M5 scope, indicators-only window, stack precision, architecture rules) unchanged.

**Testability needs (updated):** Nothing from you. Plus a new requirement on the result: the validator agent must be able to test and validate it independently. That means the app ships a headless verification mode (no window needed) that regenerates named places, checks determinism and border agreement, and dumps indicator snapshots the validator can read. The window itself is for you; the headless mode plus these snapshots are for the validator and the automated checks. Proving the validator can run this mode is a separate preflight step before implementation starts.

Rest of v3 (goal, non-goals, acceptance criteria, test plan) stands as written.

---
Please reply `approved` on this Issue if v4 matches your notion, or tell me what to change.


---

approved

---

Preflight: PASS

Issue: #35 — Procedural universe game MVP (spec v4 approved).
Scope for gate time: M1–M5 indicators-only Bevy window + headless verification mode (v4 addition). v3 holds goal/non-goals/acceptance/test plan; v4 adds headless-mode testability requirement; owner replied `approved`.

Gap list: needed vs already proven

Needed to validate at gate time (merged commit SHA):
1. Read repo tree at a commit SHA — ALREADY PROVEN (registry-adjacent; re-demonstrated today, see Evidence).
2. Read rendered/file contents at a commit SHA: ladder doc, architecture/stack doc, headless-mode indicator snapshots, headless check output — ALREADY PROVEN (registry entry: "read a rendered file on GitHub at a commit SHA", proven #30 via raw file URL + contents API at commit f60bf56; re-demonstrated today at ba0d681).
3. Headless verification mode + its dumped snapshots/check output — NOT YET BUILT (it is the implementation deliverable, not a preflight blocker). At gate time the validator reads its output; it does not execute cargo itself.
4. Desktop window fly-through (test plan steps 2–3) — OWNER-SIDE, no validator access needed; validator covers the headless side.
5. Rust toolchain / accounts / devices / services — NOTHING NEEDED from validator or owner. Spec testability needs: "Nothing from you. Desktop window + headless checks" (v3) + headless mode requirement (v4). No secrets, accounts, or devices involved.

Nothing owner-only is missing. No owner access request.

Steps taken + Evidence

1. Read approved spec via `gh issue view 35 --json comments` (8 comments, all by owner login `taftary`; repo owner is `taftary` per `gh repo view`):
   - v1 spec, v2 (indicators-only window + pinned stack + SOLID/DRY), v3 (scope extended M1–M5, indicators only, determinism + border-agreement checks), v4 (testability-only change: app ships headless verification mode that regenerates named places, checks determinism + border agreement, dumps readable indicator snapshots; proving the validator can run this mode is this preflight step), then owner `approved` (2026-10-01T13:06:02Z).
2. Read registry `docs/validator-capabilities.md`: one entry — "read a rendered file on GitHub at a commit SHA", proven #30 on 2026-10-01.
3. Verified GitHub access from this shell: `gh api repos/taftary/astrolith/contents/docs/validator-capabilities.md?ref=ba0d681` returned the file (html_url `.../blob/ba0d681/docs/validator-capabilities.md`, blob sha 300eb5d...); `curl.exe -sL https://raw.githubusercontent.com/taftary/astrolith/ba0d681/docs/validator-capabilities.md` returned the registry content. HEAD is `ba0d681` (`git rev-parse HEAD`); `git ls-tree -r ba0d681 --name-only` lists the tree (no Cargo.toml / src / snapshots yet — game app + headless mode do not exist yet, as expected pre-implementation).
4. Rust toolchain (rustc/cargo): established by main agent; not independently executable from this restricted shell (gh/curl/browser reads only) and not required for the verdict — gate-time validation reads dumped snapshots + check output, never runs cargo.

What the validator will execute at gate time (merged commit SHA `<MERGE_SHA>`)

1. `git ls-tree -r <MERGE_SHA>` (or contents/tree API) to confirm deliverables exist: ladder doc, architecture/stack doc, headless-mode source, dumped indicator snapshots, check output/log.
2. Read at `<MERGE_SHA>` via raw file URL + contents API (the proven capability):
   a. Indicator snapshots, one per named place along the M1–M5 chain (cosmic web → galaxies → stars → planet surface; exact paths as implemented) — expect grids/vectors/points data only, zero mesh/texture references.
   b. Headless verification output (`--verify` log / check report) — expect per-check lines: determinism (same place twice → byte-identical) PASS, border-agreement (neighbors agree) PASS, overall exit 0 / ALL GREEN.
   c. Frozen ladder + symbol table doc — expect 15 levels + every symbol from the notion, no contradictions.
   d. Architecture/stack doc — expect workspace crates (core, rendering, app), explicit feature flags, pinned deps with justification, rules (headless core, one plugin per feature, DRY, SOLID, documented).
3. Confirm CI green for `<MERGE_SHA>` (required merge gate) alongside the validator pass.

PASS means: all files above readable at `<MERGE_SHA>`; snapshots present for every level in the chain and indicators-only; determinism byte-identical; border-agreement green; docs match the notion; automated checks + CI green. FAIL means: any snapshot/output/doc missing or unreadable at `<MERGE_SHA>`, any determinism mismatch, any border disagreement, any mesh content in snapshots, any doc contradiction, or any red check.


---

## Plan (AI-owned, no approval needed)

**Approach:** Cargo workspace in this repo (`universe-core` headless logic, `universe-render` Bevy indicators, `universe-app` entry + headless `--verify` mode). M1 ladder/symbol doc first, then M2 coordinates/seeding/cache/re-anchoring, then M3 points, M4 galaxies-to-stars with L5-L10 octree, M5 planet terrain LOD. Indicators only (grids, vectors, points); deterministic + border-agreement headless checks from day one.
**Affected areas:** new `crates/` tree, `docs/universe/` (ladder, stack pins, architecture rules), `.github/workflows` (extend CI: cargo check/clippy/test), README (one line pointing at the game). Workflow files untouched.
**Sequence:** workspace + CI -> M1 doc -> M2 framework + verify mode -> M3 -> M4 -> M5 -> window polish -> gate.
**Risks:** Bevy 0.19 pin verification; f32 transform boundary handled by integer-cell layer (decided); first-compile time.
**Deferred:** M6-M8, meshes, gameplay, multiplayer, consoles, physics/input crates, device spec.
No choice here changes what the product does. Staying in Working.


---

﻿Sub-issues (label `task`, off the board):

35.1=#37
35.2=#38
35.3=#39
35.4=#40
35.5=#41
35.6=#42
35.7=#43

One branch/PR `feat/35-universe-mvp` for the whole slice (pieces only testable together).


---

<!-- validator:pass sha=dfc552356f4851c5d8a58c5b2e150fa476ccacb1 -->
Validator: PASS for dfc5523
Checked: AC1 ladder freeze, AC2 L1-L11 indicators-only window + no meshes/materials/textures in render/app source, AC3 determinism predicates + tests + snapshot spot-check, AC4 workspace/pins/toolchain/stack doc/architecture rules, AC5 automated checks + CI green, notion/spec drift
Evidence:
- Notion: gh issue view 35 body (M1-M8 ladder L1-L15, symbol table l/S_l/e_l/C/sigma/sigma0/H/R/G_l/K_l/F_l/o/h/theta/theta_min + 4 formulas).
- Spec: gh issue view 35 comments = v3 (M1-M5 scope, indicators-only, determinism+border checks) + v4 headless --verify requirement + owner approved 2026-10-01T13:06:02Z + preflight PASS + plan (universe-core/render/app, --verify).
- Result: PR #44 headRefOid dfc552356f4851c5d8a58c5b2e150fa476ccacb1, state OPEN. Tree at SHA via contents/raw API: 80 entries; crates universe-core/render/app + docs/universe/ladder.md + docs/universe/stack.md + rust-toolchain.toml + Cargo.toml/lock. Diff --name-only: only workspace/crates/docs/toolchain/CI/gitignore/CLAUDE.md; no city/building/room files.
- AC1 PASS: docs/universe/ladder.md at SHA freezes 15 levels L1-L15 with names/sizes/ranges/contents verbatim vs notion, full 15-symbol table + 4 core formulas, gap note citing notion 5.4 octree.
- AC2 PASS: crates/universe-render/src/lib.rs at SHA: MIN_NAV_LEVEL=1 MAX_NAV_LEVEL=11, level_for_key 1-9/0/q, handle_level_keys + window title, draw_level_indicators grid_3d + RGB axis lines, draw_demo_points gizmo spheres from core generators; UniverseRenderPlugin + LevelNavigationPlugin. main.rs opens window L1-L11, Esc quits. Mesh grep over render lib + app main at SHA: zero Mesh/StandardMaterial/Texture/MeshMaterial/Mesh3d/Mesh2d/PbrBundle uses (only doc line stating no meshes). Stack doc deliberately-off list includes PBR/meshes.
- AC3 PASS: core verify.rs at SHA exports check_determinism (byte-compares snapshot_generated twice) + check_border (epsilon/border agreement) with unit tests; snapshot.rs exports SNAPSHOT_PRECISION=6 + snapshot_generated canonical sort/fixed-precision with 5 unit tests. Spot-check via tree: header format generated points=<n> children=<m>, test vector p 0.500000 -0.250000 0.000000 0.010000 present. No cargo run per validator constraints.
- AC4 PASS: root Cargo.toml workspace members universe-core/render/app, bevy =0.19.1 default-features=false with 15 explicit features; rust-toolchain.toml 1.97.1 + rustfmt/clippy; stack.md pins/features-justifications/rules/budgets; core Cargo.toml zero deps (headless), render deps bevy+core, app deps bevy+core+render (direction core<-render<-app); all 11 core files have no bevy/render imports (spot-checked use-statements); two plugins one-per-feature; pub items documented (spot-checked lib.rs/render/verify.rs doc comments).
- AC5 PASS: .github/workflows/ci.yml at SHA runs cargo check/clippy/test --locked + markdown/workflow/branch/auto-close guards; gh pr checks 44 = ci pass.
Drift from notion: none (M1-M5 only, L12-L15 Beyond-MVP clamp, indicators-only, no M6-M8/gameplay/multiplayer/meshes).


---

## Ready to test

Your universe MVP is merged. No code to review � just run it and fly:

**Where:** on this machine, in `C:\Users\PERSO\source\repos\astrolith` (main branch):
1. `cargo run -p universe-app` � opens the Universe MVP window.
2. Or headless: `cargo run -p universe-app -- --verify` � prints PASS per level.

**Steps:**
1. Open the window. You see a grid with colored axes (L1, observable universe).
2. Press keys 1 to 9, 0, q to travel down the ladder to planets (L11). Expected: grid + dots change per level, never 3D models, no crash.
3. Press Esc to quit.

**What you should see:** cosmic web to planet surface as grids, vectors, and dots � indicators only, exactly your spec.

Reply here with `accept` if it matches your notion, or describe what looks wrong and I will correct it.


---

## Owner test feedback (problem reported)

Owner ran `cargo run -p universe-app` on Intel UHD 620 / Vulkan. Window was created ("Universe MVP - L1"), but the log shows:

1. Real bug: `TonyMcMapFace tonemapping requires the tonemapping_luts feature` � our minimal Bevy features lack the LUTs the default camera tonemapping needs.
2. Likely benign: Steam overlay loader messages + Vulkan swapchain/semaphore validation chatter from the Intel driver.

Correction: set the indicator camera to `Tonemapping::None` (gizmo indicators need no filmic tonemapping; keeps the feature set minimal). Pending owner answer on what was actually visible (grid vs black) to judge the Vulkan noise.


---

C:\Users\PERSO\AppData\Local\Temp\opencode\validator-46.md

---

<!-- validator:pass sha=b024b216c6473e9af24a2d8e18bca05e8844c86b -->
Validator: PASS for b024b21
Checked: diff scope (single file, no features/meshes/scope creep); fix correctness for TonyMcMapFace LUT failure (Tonemapping::None bypasses tonemapping_luts); import path valid in pinned Bevy 0.19.1 tree (bevy_core_pipeline enabled + CI compile); CI green for PR #46; spec v3/v4 + notion match preserved (indicators-only L1-L11 window, no meshes/materials/textures)
Evidence:
- Notion: gh issue view 35 --json body — procedural universe game MVP, 15-level ladder L1-L15 with symbol table (l/S_l/e_l/C/sigma/sigma0/H/R/G_l/K_l/F_l/o/h/theta/theta_min + 4 formulas), M1-M8 milestones.
- Spec: gh issue view 35 --json comments (14 comments) = v3 (M1-M5 scope, indicators-only grids/vectors/points, no meshes, determinism+border checks) + v4 (headless --verify requirement, unchanged goal/non-goals) + owner `approved` 2026-10-01T13:06:02Z + preflight PASS + plan; owner test-feedback 2026-10-01T14:32:30Z reporting `TonyMcMapFace tonemapping requires the tonemapping_luts feature` on Intel UHD 620 / Vulkan with window "Universe MVP - L1", prescribing `Tonemapping::None` to keep feature set minimal.
- Result: PR #46 https://github.com/taftary/astrolith/pull/46 headRefOid b024b216c6473e9af24a2d8e18bca05e8844c86b (matches gate SHA), base main, state OPEN, mergeable MERGEABLE, changedFiles 1, additions 6, deletions 0.
- Diff: gh pr diff 46 + gh api pulls/46/files — only crates/universe-render/src/lib.rs: `use bevy::core_pipeline::tonemapping::Tonemapping;` + `Tonemapping::None` on indicator camera + rationale comment citing minimal-features pin and docs/universe/stack.md. No other files, no Mesh/StandardMaterial/Texture/PbrBundle/mesh-feature additions.
- Import validity: root Cargo.toml bevy = "=0.19.1" default-features=false with `bevy_core_pipeline` in 15 explicit features; Cargo.lock bevy 0.19.1 + bevy_core_pipeline 0.19.1; docs/universe/stack.md confirms bevy_core_pipeline -> CorePipelinePlugin and deliberately-off PBR/meshes. CI `ci` SUCCESS 2026-10-01T14:38:18Z (cargo check/clippy/test --locked) proves the import compiles in the pinned tree.
- CI: gh pr checks 46 = `ci pass 3m15s`, statusCheckRollup conclusion SUCCESS.
- Notion/spec match: grep over crates/universe-render/src/lib.rs finds zero Mesh/StandardMaterial/Texture/MeshMaterial/Mesh3d/Mesh2d/PbrBundle uses (only doc line stating no meshes + comment mentioning tonemapping_luts avoidance); crates/universe-app grep finds zero Mesh/tonemapping additions; MIN_NAV_LEVEL=1 MAX_NAV_LEVEL=11, level_for_key 1-9/0/q, grid_3d + RGB axes + gizmo spheres unchanged. Indicators-only preserved.
- Headless limit (honest): cannot verify actual pixels on Intel UHD 620 / Vulkan headlessly — whether grid vs black was visible, whether Vulkan swapchain/semaphore validation chatter or Steam overlay loader messages persist, and visual tonemapping difference for gizmos. Verified code-correctness + CI only; owner re-run on Intel Vulkan required for visual confirmation.
Drift from notion: none


---

## Ready to test (correction round 1)

Fixed and merged. Re-test on your machine:

**Where:** same repo, main branch, `cargo run -p universe-app`.

**Steps:**
1. Run it. The TonyMcMapFace error should be gone from the log.
2. Fly keys 1-9, 0, q. Expected: grids + dots per level, no crash.
3. Tell me what you see (grid visible or black screen?) plus any remaining error lines � the Vulkan swapchain chatter needs your eyes to judge.

Reply here with `accept` if it matches your notion, or describe what is wrong.


---

Retest round 1 feedback recorded: TonyMcMapFace error GONE (fix works). Remaining: Intel Vulkan swapchain/semaphore validation chatter; app runs ~8s and exits cleanly on window close. Still need eyes: grid visible or black? Next experiment below.

---

## Correction round 2 (owner retest)

R1 fix verified (LUT error gone). DX12 log fully clean, yet window shows nothing on either backend � rendering issue, not driver.

Root cause found in pinned sources: `GizmoRenderPlugin` only installs the 3D line pipeline (`LineGizmo3dPlugin`) when the `bevy_pbr` feature is on AND `PbrPlugin` is loaded. Our minimal pin lacks `bevy_pbr`, so gizmo lines have no render pipeline. Fix: add `bevy_pbr` to the pinned features (still no meshes in our code � indicators only holds).


---

<!-- validator:pass sha=fb181be477599b995a8b2bf51a118c5b80bf9b19 -->
Validator: PASS for fb181be
Checked: diff scope (only Cargo.toml + Cargo.lock + docs/universe/stack.md, bevy_pbr added, no code changes); root-cause fix correctness (3D gizmo line pipeline gated on PbrPlugin — claim matches bevy 0.19.1 source); no meshes added to our code; pin stays exact (=0.19.1, default-features=false); stack doc justification present and accurate; CI green for PR #48; spec v3/v4 + notion match preserved (indicators-only L1-L11 window); R1 Tonemapping::None preserved
Evidence:
- Notion: gh issue view 35 body — procedural universe game MVP, 15-level ladder L1-L15 with symbol table (l/S_l/e_l/C/sigma/sigma0/H/R/G_l/K_l/F_l/o/h/theta/theta_min + 4 formulas), M1-M8 milestones.
- Spec: gh issue view 35 comments = v3 (M1-M5 scope, indicators-only grids/vectors/points, no meshes, determinism+border checks) + v4 (headless --verify requirement) + owner approved 2026-10-01T13:06:02Z + preflight PASS + plan; test-feedback LUT error fixed in R1; R2 comment 2026-10-01T15:04:01Z diagnosing black window on clean DX12 log, root cause GizmoRenderPlugin installs 3D line pipeline only with bevy_pbr/PbrPlugin, fix bevy_pbr with no meshes in our code.
- Result: PR #48 https://github.com/taftary/astrolith/pull/48 headRefOid fb181be477599b995a8b2bf51a118c5b80bf9b19 (matches gate SHA), base main, state OPEN, mergeable MERGEABLE, changedFiles 3, additions 57, deletions 1.
- Diff: gh pr diff 48 + gh api pulls/48/files filenames = exactly Cargo.lock + Cargo.toml + docs/universe/stack.md. Cargo.toml hunk adds only "bevy_pbr" line to features. Cargo.lock hunks add bevy_pbr 0.19.1 + bevy_mikktspace 1.0.0 + wiring (bevy_material/math/mesh/pbr deps). stack.md hunk adds table row for bevy_pbr + changes deliberately-off line from "PBR/meshes" to "meshes". Zero .rs files touched, so no code/mesh changes possible in this PR.
- Pin: Cargo.toml at SHA (via contents API decode) = bevy version "=0.19.1", default-features=false, 16 features (15 prior + bevy_pbr). Exact pin and minimal-features posture preserved.
- Mechanism match: raw bevy v0.19.1 crates/bevy_gizmos_render/src/lib.rs confirms stack.md wording — `#[cfg(feature = "bevy_pbr")] mod pipeline_3d;` and `#[cfg(feature = "bevy_pbr")] if app.is_plugin_added::<bevy_pbr::PbrPlugin>() { app.add_plugins(pipeline_3d::LineGizmo3dPlugin); ... }`. Doc comment "Requires to be loaded after PbrPlugin or SpriteRenderPlugin". stack.md row "`PbrPlugin`: required host for the 3D gizmo line pipeline (`LineGizmo3dPlugin` only installs when `PbrPlugin` is loaded; our code still creates no meshes)" matches both the compile-time feature gate and runtime plugin gate. Runtime side satisfied because universe-app main.rs at SHA uses DefaultPlugins (verified) which loads PbrPlugin when bevy_pbr is enabled.
- No meshes: grep over crates/universe-render/src/lib.rs at SHA (405 lines) for Mesh|StandardMaterial|Texture|MeshMaterial|Mesh3d|Mesh2d|PbrBundle|PbrPlugin = zero hits; same grep over crates/universe-app/src/main.rs at SHA = zero hits. R1 preserved: Tonemapping::None import + spawn_indicator_camera + rationale comment still present at SHA; MIN_NAV_LEVEL=1 MAX_NAV_LEVEL=11, grid_3d + RGB axes + gizmo spheres unchanged (PR touches no src).
- CI: gh pr checks 48 = `ci pass 5m43s` (actions run 36882339543/job/110436976393). Proves bevy_pbr feature resolves and compiles in pinned tree with --locked.
- Notion/spec match: M1-M5 indicators-only scope unchanged; deliberately-off list still excludes audio/UI/text/sprites/meshes/glTF/picking/etc; budgets/architecture sections untouched.
- Headless limit (honest): cannot verify actual pixels — whether grid + dots now render on Intel UHD 620 Vulkan/DX12, frame presentation, or 60fps feel. Verified code-correctness + CI only; owner re-run of cargo run -p universe-app required for visual confirmation.
Drift from notion: none


---

Correction round 2 merged (CI green, validator PASS for fb181be, identical tree). Re-test: `cargo run -p universe-app` (DX12 was clean for you — keep it). Expected this time: white grid + colored axes + dots. Reply `accept` or report what you see.

---

## Owner test feedback (problem reported, round 2 retest)

R2 verified: grid appears (after a first-run shader-compile pause). But then nothing moves � switching keys jumps discretely between levels with no visible travel. The notion demands continuous travel ("travel feels constant at every level", camera model `v = k_speed � h`, level from `log10(h)`).

Correction round 3: smooth exponential zoom (wheel / +/- keys, constant-feel per the spec formula), active level derived from altitude, interpolated indicator scale, keys jump to rungs. Startup shader pause noted as accepted one-time cost.


---

<!-- validator:pass sha=87e57b53a1da679fea1b24ad09077e2296cd34c6 -->
Validator: PASS for 87e57b5
Checked: diff scope (only lib.rs + main.rs); zoom state/keys/wheel wiring vs R3 prescription; level derivation + interpolation sanity (endpoints L1@rung10 L11@rung0, monotonic, all rungs reachable via new zoom tests); zoom-sweep verify present in --verify; no meshes added; indicators-only holds; CI green for PR #50; spec v3/v4 + result match notion section 7 (v=k*h, level from log10(h), continuous travel)
Evidence:
- Notion: gh issue view 35 --json body — procedural universe game, section 7 camera model v=k*h, level from log10(h), continuous travel fantasy; 15-level ladder L1-L15, symbol table, M1-M8.
- Spec: gh issue view 35 --json comments — v3 (M1-M5 indicators-only, determinism+border checks) + v4 (headless --verify) + owner approved 2026-10-01T13:06:02Z + preflight PASS + R3 prescription comment 2026-10-01T15:32:00Z (smooth exponential zoom wheel/+/- keys constant-feel per spec formula, level from altitude, interpolated scale, keys jump to rungs).
- Result: PR #50 https://github.com/taftary/astrolith/pull/50 headRefOid 87e57b53a1da679fea1b24ad09077e2296cd34c6 (matches gate SHA), base main, state OPEN, mergeable MERGEABLE, changedFiles 2, additions 219, deletions 36.
- Diff scope: gh pr diff 50 + gh api pulls/50/files filenames = exactly crates/universe-render/src/lib.rs + crates/universe-app/src/main.rs.
- Wiring (lib.rs at SHA via contents API): ZoomState{position:10.0=L1} + init_resource, apply_zoom_travel chained after handle_level_keys; MouseWheel (Line y + Pixel y/24.0) delta -= lines*WHEEL_STEP (0.5/rung, wheel-up dives); ArrowUp/Equal dive KEY_RATE 3.0 rungs/s, ArrowDown/Minus rise; clamp 0.0-10.0; comment cites notion v=k*h (linear rung steps = exponential altitude steps). handle_level_keys sets zoom.position=rung_for_level(level) for 1-9/0/q (level_for_key unchanged), Esc quits; apply_zoom_travel syncs CurrentLevel + window title on change, moves camera on view diagonal distance=GRID_CELLS*spacing.
- Level/interp (lib.rs at SHA): rung_for_level=11-level.get(); level_at_position=Level::new(11-round(clamp 0-10)); spacing_at=log-interp (ln/exp) between rung indicator_spacings; AXIS_SCALE 5.0, MARKER_SCALE 0.06 replace per-level fns; draw_level_indicators + draw_demo_points use spacing_at(position) for grid/axes/extent/radius + camera distance.
- Zoom tests (lib.rs at SHA, mod zoom_tests, 4 tests): rungs_hit_endpoints_and_every_level (L1@10, L11@0, all 0-10 round-trip rung_for_level), sweep_visits_all_levels_monotonically (0.0-10.0 step 0.25, never rises, all 11 seen), spacing_matches_rungs_and_grows_smoothly (rung endpoints == indicator_spacing within 1e-6, monotonic growth), positions_clamp_into_range (99->L1, -99->L11, spacing>0). Endpoints/monotonic/reachability all asserted.
- Zoom-sweep verify (main.rs at SHA): run_verify calls verify_zoom_sweep(), prints ZOOM-SWEEP endpoints=L11,L1 all-levels PASS/FAIL; sweeps 0.0-10.0 step 0.25 requiring endpoints L11@0/L1@10, never-rises, all 11 seen; returns pass flag ANDed into VERIFY-OK/FAIL.
- No meshes: Select-String over both files at SHA for Mesh|StandardMaterial|Texture|PbrBundle|Mesh3d|Mesh2d (case-insensitive) = zero type uses; sole render-lib hit is doc line stating no meshes by design. Diff adds no mesh/material/texture strings.
- Indicators-only: lib.rs at SHA draws gizmos only — grid_3d + 3 RGB lines + gizmo spheres; Tonemapping::None import+use preserved (R1), bevy_pbr pin untouched by this PR (R2 preserved, zero Cargo changes).
- CI: gh pr checks 50 = ci pass 8m26s (run 36886081519/job/110449605713).
- Headless limit (honest): cannot verify smooth visual motion — actual exponential-zoom feel, frame-to-frame grid growth smoothness, wheel/key repeat feel, 60fps, or pixels on Intel Vulkan/DX12. Verified code-correctness + unit tests + CI only; owner re-run of cargo run -p universe-app required for visual confirmation.
Drift from notion: none (M1-M5 indicators-only preserved, continuous travel added per notion section 7 + R3 prescription; no cities/buildings/rooms/gameplay/multiplayer/meshes).


---

Correction round 3 merged (CI green, validator PASS for 87e57b5, identical tree). Fly it: `cargo run -p universe-app`, then scroll the mouse wheel (up dives deeper) or hold ArrowUp/ArrowDown — the grid should grow/shrink smoothly and the title should walk L1 to L11. Keys 1-9/0/q still jump. Reply `accept` or report what you see.

---

## Owner test feedback (round 4; R3 zoom not yet confirmed)

 expectation: a camera that visibly travels through space toward a target planet � pick a planet in the cosmic randomness and fly to it; that flight IS the test. Discrete level generation is not travel.

Round counter: initial fail (LUT) -> R1 -> R2 (black) -> R3 (static) -> R4 (no journey). Each round fixed the reported defect; this round adds the flight test mode the owner just defined (their message is the decision to proceed).

Correction round 4: deterministic random target planet (seeded, same journey every run), Spacebar starts the flight, camera dives L1 to L11 with orbit drift so motion is visible, target beacon locked the whole way, arrival at L11 terrain. Headless flight simulation added to --verify.


## Criteria (edit + confirm with owner before PASS counts):

- [ ] C1: (derive from body above; one testable criterion per line)
  verify: command / URL / action + expected output

> Status: UNCONFIRMED — validator may run runtime checks but
> no criterion counts as MET until the owner confirms this file.
