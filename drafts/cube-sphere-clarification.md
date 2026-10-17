# Cube-Sphere Planet Engine — Intake Answers (Suggested)

Original draft: drafts/L10toL14.md
Purpose: challenge/clarify before taking into Issue or moving to docs/.

## 1) Problem vs solution
- Problem: current icosphere/patch direction is showing seams/jitter/precision issues at planetary scales; we need continuous navigation from orbit to ground (no rescaling, stable transforms) with stable visuals/physics.
- This reads like a technical design/spec. Suggest: take as a notion Issue only if we plan to implement now. Otherwise move to docs/design/cube-sphere-planet-engine.md as permanent design (drafts/ stays for intake).

## 2) Scope vs existing work
- Relation to #729/#742: consistent direction (remove icosphere-centric L10–L13, true-size travel, no portals/rescaling). Treat cube-sphere as replacement direction, not parallel experiment unless stated.
- Rewrite vs extend: replace terrain/LOD core for planetary scale. Keep travel/addresses/teleports and dive-level semantics compatible where possible.
- Scope boundary (first deliverable): foundation only — (1) double-precision global coords, (2) camera-relative rendering + floating origin, (3) 6-face cube-sphere with distortion-corrected mapping, (4) per-face quadtrees + screen-space error LOD. Everything else in follow-ups.

## 3) Acceptance & testability
- Acceptance (observable): (a) smooth descent orbit→ground with no visible popping/cracks, (b) no camera/entity jitter at ~1cm inspection, (c) stable across cube-face seams and poles, (d) deterministic given same seed/coords/LOD state.
- Smallest slice: single cube face with SSE LOD, frustum culling, camera-relative + floating origin, basic mesh generation.
- Testability: unit tests for math (cube→sphere, mapping validation, edge/neighbor logic, determinism). Headless GL helpful for mesh metrics; seam/popping likely need a running window for visual checks — document which need window vs headless.
- Budgets: "acceptable without stutter" first round. Track visible patches/triangles via debug overlay (target ~1–2k patches / 2–3M tris). No hard FPS floor now; avoid regressions.

## 4) Technical choices & constraints
- Cube mapping: evaluate >=2 candidates (spherified cube, tangent-adjusted). Propose numeric threshold (e.g. face-area/edge deviation < 10–15%) and vertex-spacing ratio across centers/edges/corners. AI proposes mapping+evidence; owner breaks ties.
- Multiplayer: out of scope for first round — drop or TODO.
- Platform: desktop-first (native Bevy/WGPU). WebGL not targeted now (reversed-Z/log depth constraints) — non-goal.
- Physics: tangent-plane below ~100m, must be compatible with travel/teleports (no rescaling). Use place addresses/global double coords for teleport targets; avoid nested opens.
- Single source of truth: new shared height function API consumed by rendering and collision. Do not couple to legacy icosphere height paths.

## 5) Workflow/positioning
- Non-goals (first round): atmosphere/clouds/shadows/post-processing, multiplayer, full PBR texturing, virtual texturing, cinematic post, physics polish beyond basic collision.
- Ties to Issue: treat as replacement direction consistent with #729/#742, but create a new parent Issue (notion) if implementing now. If only recording design, move draft to docs/design/cube-sphere-planet-engine.md instead of creating Issue.
- Intake path: drafts/ are for owner intake. If moved to docs/, removal from drafts/ is fine (artifact must exist as repo doc). If taken as Issue, verbatim content + draft path recorded, then remove file.

## Recommendation
- Move drafts/L10toL14.md to docs/design/cube-sphere-planet-engine.md (permanent design reference).
- Keep this clarification draft (drafts/cube-sphere-clarification.md) in drafts/ for now, or delete after answering. Do not create Issue yet unless you want implementation to start now.