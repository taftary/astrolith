# Technical architecture instructions and repository organization

Draft written by the AI (Claude Fable, OpenCode session, 2026-10-02) at the owner's request in that session. The owner asked for more technical instructions covering the project's architecture and how files and documents are organized, asked the AI to review how advanced Rust applications are built and organized, and asked for a draft that states the goal and the migration needs in full detail. The owner's words, verbatim:

> I need to define more technical instructions in the project for the architecture of the project and how to organize files and documents review and check in internent how advanced rust application are working and organize things next in drafts folder I need to specify the goal and the migration needs with all details.

## 1. Goal

Give the project one written, enforced set of technical instructions that answers four questions for every agent and every future contributor:

1. **Architecture**: which crates exist, what each one may contain, which direction dependencies flow, and which invariants must never be broken.
2. **File organization**: where a new piece of code goes (crate, module, file), when a file must be split, how plugins and modules are named.
3. **Document organization**: where each kind of document lives (`docs/`), how decisions are recorded, and how the instruction files (`AGENTS.md`, `CLAUDE.md`) relate to them.
4. **Techniques and practices**: how errors, panics, and diagnostics are handled; how determinism and performance are kept honest; how tests are written beyond unit tests; how APIs are shaped; how dependencies, the supply chain, and CI are maintained (sections 4.9 to 4.13, added by the five review loops recorded in section 9).

Every rule is tagged **CI-checked** or **convention**, so a reader always knows whether a machine or a reviewer enforces it. Then migrate the repository to that shape **without changing what the application does**. The window, the controls, and the `--verify` / `--capture` outputs stay identical, and from M0 on a committed golden test proves it at every step.

## 2. What the repository looks like today

Observed on 2026-10-02 at commit `1f9b4ff` (main).

Strengths already in place:

- Virtual Cargo workspace, three crates under `crates/`, edition 2024, exact toolchain pin (`rust-toolchain.toml`, Rust 1.97.1), exact Bevy pin (`=0.19.1`, `default-features = false`) declared once in `[workspace.dependencies]`.
- A clear layering rule: `universe-core` (std only) <- `universe-render` (Bevy indicators) <- `universe-app` (binary). Written in `CLAUDE.md` and `docs/universe/stack.md`.
- Every public item documented; headless tests; `cargo check/clippy/fmt/test --locked` in CI on Linux and Windows; `--verify` as a deterministic headless acceptance run.
- Markdown lint, doc drift guard, workflow files check, branch and PR guards in CI.

Gaps and drift signals:

- **Pure logic lives in the render crate.** `crates/universe-render/src/lib.rs` (about 1,000 lines, one file) holds the `Universe` state machine, `dive_step`, `should_open` / `should_close`, `PreviewCache`, `replay_autopilot`, the journey types, and the level/scale label strings. None of these need Bevy, yet `universe-app --verify` can only reach them through a crate that links Bevy. This contradicts the project's own rules "headless core: all universe logic lives in `universe-core`" and "DRY: shared code moves to `universe-core`".
- **One file per crate.** `universe-render` is a single `lib.rs`; `universe-app` is a single `main.rs` (about 380 lines) that mixes argument parsing, the window, `--verify`, and `--capture`. The Bevy Book's guidance is roughly 1,000 lines per file maximum and one plugin per module; the render crate is at that limit with two plugins and eight systems in one file.
- **Rules are prose only.** "No Bevy in core", "public items documented", "no per-frame allocations in hot systems" are stated in `CLAUDE.md` but nothing in CI checks them. There is no `[workspace.lints]`, no `rustfmt.toml`, no `clippy.toml`, no dependency-direction check, no `cargo doc` check.
- **No architecture document and no decision log.** Decisions (pins, indicators only, marker-tree nesting R6, pre-entry preview R7, fixed timestep, 60 fps) are scattered across Issue comments, `docs/universe/stack.md`, `docs/universe/ladder.md` amendments, and `CLAUDE.md`. There is no single codemap that says "where is the thing that does X".
- **Two instruction files with overlapping scope.** `AGENTS.md` carries process rules; `CLAUDE.md` carries technical conventions for the `universe-*` crates. The split is sensible, but `CLAUDE.md` is tool-named, and its rules are not referenced from `docs/` or checked by the doc drift guard.
- **Hygiene.** `tmp/capture-spot-*` directories sit untracked and un-ignored (`.gitignore` only covers `tmp/agent-shared/*`). `crates/universe-render/Cargo.toml` carries a stale comment ("unused until then") about a dependency that is now used heavily. `scripts/` and `.agent/` have no README explaining their subfolders.
- **Techniques (detail in the "State today" paragraph that opens each of sections 4.9 to 4.13).** Output vocabulary is an unstated interface with the validator; transcendental math is platform-dependent and never compared across the two CI platforms; the binary has no tests and the headless protocol no golden file; positions travel as bare arrays in two unit frames with 60 unchecked `as` casts; 423 locked packages are never scanned, CI actions are tag-pinned, and every CI run rebuilds Bevy from scratch.

## 3. What advanced Rust and Bevy projects do

Reviewed on 2026-10-02. Only the points that apply to this project are kept. This list covers architecture and organization; the technique sections 4.9 to 4.13 name their own sources inline, next to the rules they justify.

- **Bevy Book, Project Organization** (<https://bevy.org/learn/book/modular-architecture/project-organization/>): split by domain, not by kind ("do not group components and systems"); one plugin per module and one plugin per crate as the default; split files beyond about 1,000 lines; `pub(crate)` is the right default visibility, `pub` is reserved for deliberate shared interfaces; grow the structure gradually, do not over-architect up front.
- **Bevy Book, Testing Bevy Apps** (<https://bevy.org/learn/book/development-practices/testing/>): prefer tests that need no ECS at all; then a raw `World`; then `run_system_once`; then a `Schedule`; then stepping an `App` with `MinimalPlugins`. Keep graphics decoupled so headless tests stay possible, and "create a headless integration test immediately and religiously ensure it continues working". Fixed timestep and fixed seeds make output deterministic.
- **Bevy Book, Building Bevy's Ecosystem** (<https://bevy.org/learn/quick-start/plugin-development>): `default-features = false` on Bevy; check duplicate dependencies with `cargo tree` or `cargo-deny`.
- **Bevy best practices** (<https://github.com/tbillington/bevy_best_practices>): plugins as the unit of organization; explicit system ordering; events to decouple subsystems; a crate-level `prelude` module.
- **`ARCHITECTURE.md`** (matklad, <https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html>; example: rust-analyzer `docs/book/src/contributing/architecture.md`): one short document with a bird's-eye view, a codemap ("where is the thing that does X"), explicit **architecture invariants** (often absences: "core knows nothing about Bevy"), and marked **API boundaries**. Name files and types, do not link them. Revisit a few times a year rather than keeping it in lockstep with code.
- **Architecture Decision Records, MADR** (<https://adr.github.io/madr/>): `docs/decisions/NNNN-title.md`, each with context, options, decision, consequences; superseded records are kept and marked, never deleted.
- **Cargo workspaces** (<https://doc.rust-lang.org/cargo/reference/workspaces.html>): `[workspace.dependencies]` as the single source for versions (already done here); `[workspace.lints]` with `[lints] workspace = true` in each member so every crate gets the same lint policy; `[workspace.package]` for shared metadata.
- **Clippy lint groups** (<https://doc.rust-lang.org/clippy/lints.html>): enable the default groups; cherry-pick from `pedantic`; never enable `restriction` as a whole.
- **`cargo-deny`** (<https://github.com/EmbarkStudios/cargo-deny>): advisories, licenses, duplicate versions, and `[bans]` with `wrappers` to allow a dependency only from named crates. Projects such as torrust-tracker use it to enforce workspace layer boundaries.
- **Large Rust repositories** (Zed `zed-industries/zed`, Veloren `veloren/veloren`, rust-analyzer): `crates/` for all packages; `docs/` as an mdBook or plain Markdown tree; `script/` or `scripts/` for repo tooling; root `rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`; `CONTRIBUTING.md`; crates named by domain (`common`, `client`, `server`, `world`), with utility crates treated as implementation detail; a thin binary crate on top of library crates.

## 4. Target state

### 4.1 Repository layout

```text
astrolith/
  AGENTS.md                 process rules (unchanged role); adds one link to docs/engineering.md
  CLAUDE.md                 thin pointer to AGENTS.md and docs/engineering.md (no rules of its own)
  README.md                 unchanged
  Cargo.toml                workspace: members, package, dependencies, lints, profiles
  Cargo.lock
  rust-toolchain.toml
  CONTRIBUTING.md           new: five lines pointing at AGENTS.md and docs/ (no rules of its own)
  LICENSE                   new, after open question 9
  rustfmt.toml              new: formatting policy (edition, width), committed
  clippy.toml               new: clippy configuration (thresholds, test allowances), committed
  deny.toml                 new (optional, see open question 3): advisories, licenses, bans
  typos.toml                new: spell-check configuration
  .editorconfig             new: whitespace and newline rules editors apply before CI does
  crates/
    universe-core/          pure logic, std only; gains nav/ and labels; proptest-regressions/ committed
    universe-render/        Bevy plugins only; one module per plugin
    universe-app/           binary; thin main.rs plus verify.rs, capture.rs, protocol.rs
      tests/golden/         committed --verify and --capture baselines (section 4.11)
  docs/
    README.md               new: index of docs/, what lives where
    ARCHITECTURE.md         new: bird's-eye view, codemap, invariants, boundaries
    engineering.md          new: the technical instructions (this draft's sections 4.2 to 4.13)
    decisions/              new: ADRs in MADR form, NNNN-title.md
    universe/               domain docs: ladder.md (frozen), stack.md (pins and features)
    workflow.md             process, unchanged
    board.md                process, unchanged
    validator-capabilities.md  process, unchanged
    plans/                  historical plans, unchanged
  drafts/                   owner intake, unchanged
  scripts/                  Python agent tooling; gains README.md
  .agent/                   machine config and evidence; gains README.md
  .github/                  CI; gains the new checks, SHA-pinned actions, cache, dependabot.yml
  .opencode/                agents and skills, unchanged
  tmp/                      fully ignored except README files
```

### 4.2 Crate architecture rules

| Crate | May contain | Must not contain | Depends on |
|---|---|---|---|
| `universe-core` | Deterministic universe logic: seeds, coordinates, generation, nesting, navigation state machine and math, labels, snapshots, verification predicates, headless journey replay | Anything from Bevy, wgpu, winit, or any rendering or windowing crate; any I/O beyond returning strings | std only |
| `universe-render` | Bevy plugins, resources that wrap core types, systems, gizmo drawing, input handling, camera, colors | Universe logic that could run without a window; meshes, materials, textures (spec v4: indicators only) | `bevy`, `universe-core` |
| `universe-app` | `main`, CLI flag parsing, window setup, `--verify` and `--capture` drivers | Universe logic; drawing code | `bevy`, `universe-core`, `universe-render` |

Invariants (to be written into `docs/ARCHITECTURE.md` as **Architecture Invariant** callouts, and checked by CI where marked):

- Dependency direction is `core <- render <- app`, never otherwise. **CI-checked** (dependency-direction step, section 4.6).
- `universe-core` compiles and tests with no Bevy in its dependency tree. **CI-checked**.
- Everything `--verify` needs is reachable from `universe-core` alone. **CI-checked** once the move in M3 lands (the verify driver is compiled with core only).
- `universe-render` creates no meshes, materials, or textures. Convention, reviewed in PRs.
- Determinism: the same root seed and path produce byte-identical snapshots on every run and platform. **CI-checked** (`--verify` and existing tests).
- Bounded memory: at most `2 + PREVIEW_CAP` generations alive. **CI-checked** (existing tests).
- Fixed timestep for simulation systems; no per-frame allocation in hot systems. Convention, reviewed in PRs.
- `docs/universe/ladder.md` changes only with owner approval. Unchanged rule.

API boundaries: `universe-core` is the only crate whose public API is meant to be stable inside the workspace; `universe-render` exposes exactly its plugins plus the few helpers `universe-app` needs; `universe-app` exposes nothing.

### 4.3 Organization inside a crate

- Split by domain, not by kind. A module owns its types, systems, and tests together. No `components.rs` / `systems.rs` buckets.
- One plugin per module in `universe-render`. The module name is the plugin's domain (`camera`, `input`, `draw`), the plugin type is `<Domain>Plugin`. A parent plugin may group them and must declare the system order explicitly with `SystemSet`s or `.chain()`, never implicitly.
- The current struct-based plugin style (`UniverseRenderPlugin`, `DivePlugin`) stays; do not mix in function plugins.
- File size: aim for 300 to 600 lines; split before 1,000. Tests count.
- Visibility: `pub(crate)` by default, `pub` only for items another crate uses. Public items carry a doc comment (lint-enforced, section 4.6).
- Each crate root (`lib.rs` / `main.rs`) begins with a crate-level doc comment that lists its modules in one line each. This is the per-crate table of contents.
- Unit tests live in the file they test (`#[cfg(test)] mod tests`). Integration tests that exercise a crate's public API live in that crate's `tests/`. The headless journey replay is the integration test of record and stays in core.
- No `examples/`, `benches/`, or `xtask` until a concrete need exists. Python scripts in `scripts/` remain the repo tooling; `xtask` is adopted only if a task needs the Rust toolchain itself.

### 4.4 Documentation organization

| Document kind | Location | Owner of changes |
|---|---|---|
| Process (workflow, board, validator capabilities) | `docs/workflow.md`, `docs/board.md`, `docs/validator-capabilities.md` | Sync and Governance in `docs/workflow.md` |
| Architecture codemap and invariants | `docs/ARCHITECTURE.md` | AI; revisited at the end of each parent Issue that touches crates |
| Technical conventions | `docs/engineering.md` | AI proposes, owner approves changes to rules marked owner-protected |
| Decisions | `docs/decisions/NNNN-title.md` (MADR) | AI records; owner decisions cite the Issue comment |
| Domain reference (ladder, stack) | `docs/universe/` | `ladder.md` frozen; `stack.md` pins and features |
| Historical plans | `docs/plans/` | Read-only |
| Specifications, plans, validation evidence | On the Issue (and `.agent/validation/`), never in `docs/` or `drafts/` | Unchanged |
| Owner intake | `drafts/` | Owner |

Rules:

- `docs/README.md` is the index: one line per file or folder, kept current by the doc drift guard (every `docs/*.md` and `docs/*/` must be listed).
- `docs/ARCHITECTURE.md` names files, modules, and types but does not link them (links go stale; symbol search does not). It has four sections: Bird's-eye view, Entry points, Codemap (per crate, with invariants and boundaries), Cross-cutting concerns (determinism, bounded memory, testing, error handling, logging).
- One ADR per architecturally significant decision. Status is one of proposed, accepted, superseded by NNNN. Superseded records stay. Initial back-fill (section 5, M1) records the decisions already taken so the log starts complete.
- `docs/universe/stack.md` keeps the pins and the Bevy feature table; its "Workspace crates and dependency rules" and "Architecture rules" sections move to `docs/ARCHITECTURE.md` and `docs/engineering.md`, leaving a pointer. See open question 6.

### 4.5 Instruction files: the project's memory

`docs/workflow.md` already states the principle: "all of them are written in the repository, so the work never depends on what an agent remembers between sessions." Today that is true for the process and false for the technical practice: the research and the rules in this draft exist only here and, once taken, on the Issue. An agent starting a session next month would not see them. This section makes the instruction files the single, organized, complete home of everything this draft establishes, so that all of it is considered in every future session without anyone having to remember it exists.

**Principle: one home per rule, every rule has a home, every file has one job.**

| File | Job | Size discipline | Who reads it |
|---|---|---|---|
| `AGENTS.md` | Process rules (unchanged) plus a short "Technical conventions" block: the always-on technical rules that must never be missed even when nothing else is loaded, and the pointers to the files below | One screen; a rule enters only if forgetting it would break an invariant or waste a session | Every agent, automatically, at session start |
| `CLAUDE.md`, `CONTRIBUTING.md` | Pointers only, for tools and humans that look for those names | Five lines each | Tool-specific loaders; newcomers |
| `docs/engineering.md` | The complete technical conventions: every rule from sections 4.2 to 4.13, with its tag, its probe, its sources, and its rule ID | As long as it needs to be, organized so that any question is one heading away; no narrative, no history | Agents when implementing, reviewing, or validating; loaded by the skills named below |
| `docs/ARCHITECTURE.md` | Where things are and what must never happen (codemap, invariants, boundaries) | Short; revisited, not synchronized | Anyone asking "where is the thing that does X" |
| `docs/decisions/` | Why: one ADR per significant decision, including the research that led to it | One decision per file; superseded files stay | Anyone asking "why is it like this" |
| `docs/README.md` | Index of all of the above | One line per entry | Entry point |

**Rule IDs.** Every rule in `docs/engineering.md` carries a stable identifier in the style of the Rust API Guidelines (`C-NEWTYPE`): `E-CORE-NO-BEVY`, `E-NO-PANIC`, `E-STDOUT-PROTOCOL`, `E-GOLDEN`, `E-CAST`, `E-PIN-ADR`, and so on. IDs let a pull request description, an ADR, a validator report, a `#[expect(..., reason = "E-CAST: f64 to f32 at the render boundary")]`, or a code comment cite a rule in four characters instead of paraphrasing it, and let the doc drift guard check that a cited ID exists. An ID is never reused; a retired rule keeps its ID with the note "retired by ADR NNNN".

**Completeness: this draft lands entirely, not partially.** M1 produces a traceability table on the Issue with one row per rule in sections 4.2 to 4.13 and the file and ID where it landed (`engineering.md` for rules, `ARCHITECTURE.md` for invariants, an ADR for decisions, `ci.yml` for checks). A rule with no row is a defect of M1, not a judgment call. The research itself is kept too: section 3 and the per-section sources become the "References" appendix of `docs/engineering.md` (grouped by topic, with the date reviewed), and the revision notes of section 9 become the "Context" of ADR 0009, so a future agent can see not only the rule but what was read and what was found in the code when the rule was written.

**Loading: the files are read because the process says so, not because someone remembers.**

- `AGENTS.md` names `docs/engineering.md` and `docs/ARCHITECTURE.md` under References and carries the always-on block, so the pointers reach every session.
- The `implementation` skill gains a step before coding: read `docs/engineering.md` sections relevant to the sub-issue, and run the CI-checked list locally (the same commands CI runs) before opening the pull request. The `technical-validation` skill gains a line in its report: "rule compliance", listing any `E-` ID the change touches and how it was checked. The `planning` skill cites the `E-` IDs a plan relies on. These are skill changes, which `docs/workflow.md` lets the AI propose freely; the one line added to `AGENTS.md` is an instruction change and is part of what the owner approves with this specification.
- The validator agent's inputs stay as `docs/workflow.md` defines them (notion, approved spec, result). `docs/engineering.md` is not a fourth input; the spec's acceptance criteria name the `E-` rules that matter for that Issue, so the validator still checks against the spec.

**Keeping it current.**

- Every pull request that changes a crate answers one question in its template: "Does this change add, change, or retire a rule in `docs/engineering.md`? If yes, which ID and in which commit." Rules therefore evolve in the same pull request as the code, the way `docs/workflow.md` already requires for process changes.
- Each section of `docs/engineering.md` carries a "last reviewed" date. The sections are re-reviewed against current sources (the Bevy Book, the Rust API Guidelines, Clippy, Cargo) when the Bevy or Rust pin changes, and otherwise once a year; the review is a `chore/` pull request whose body lists what was re-checked and what changed, which is exactly the search, review, investigate, update, note, refactor loop this draft used, made routine.
- **CI-checked**: the doc drift guard verifies that `docs/engineering.md` contains every heading in the fixed section list below and that every `E-` ID cited anywhere in the repository (code, docs, skills, pull request template) exists in `docs/engineering.md`; that `CLAUDE.md` and `CONTRIBUTING.md` contain only pointers; and that `AGENTS.md` names the two documents.

**Section list of `docs/engineering.md`, in this order:** toolchain and pins; workspace layout; crate rules; module and file rules; naming, visibility, and API design; documentation; testing ladder and techniques; Cargo features policy; error handling and diagnostics; headless protocol; numerics and determinism; performance and profiling; lint and format policy; dependencies and supply chain; CI; "adopt when needed" (tools deliberately not adopted, each with its trigger); how to add a crate, a plugin, a feature, a dependency, a decision; references.

**Always-on block proposed for `AGENTS.md`** (the owner approves the exact wording with the specification):

- `universe-core` never depends on Bevy or any rendering crate (`E-CORE-NO-BEVY`).
- No `unwrap`, `expect`, or `panic!` outside tests (`E-NO-PANIC`).
- stdout is the machine protocol; the words error, warn, panic, deprecated, FAILED are reserved for real problems (`E-STDOUT-PROTOCOL`).
- Golden files under `tests/golden/` are rewritten only with `UPDATE_GOLDEN=1` and only when the pull request explains the change (`E-GOLDEN`).
- Exact pins (Rust, Bevy) change only through an ADR (`E-PIN-ADR`).
- Before implementing, read `docs/engineering.md`; before asking where something lives, read `docs/ARCHITECTURE.md`.

### 4.6 Enforcement

Every rule in `docs/engineering.md` is tagged **CI-checked** or **convention**. New CI steps (all in the existing `ci` job, Linux; `ci-windows` keeps check and test):

1. **Workspace lints.** Root `Cargo.toml` gains `[workspace.lints.rust]` (`missing_docs = "warn"`, `unsafe_code = "forbid"`, `unreachable_pub = "warn"`, `unused_qualifications = "warn"`) and `[workspace.lints.clippy]` (default groups; the panic family from section 4.9 at `deny`; the cast family, `must_use_candidate`, `missing_errors_doc`, and `missing_panics_doc` from section 4.12 at `warn`). Each crate adds `[lints] workspace = true`. CI already runs clippy with `-D warnings`, so warnings fail the build.
2. **Format and clippy configuration files.** `rustfmt.toml` (edition 2024, default style, committed so local and CI agree) and `clippy.toml` (e.g. `too-many-lines-threshold`, `cognitive-complexity-threshold`).
3. **Documentation build.** `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` catches broken intra-doc links and missing docs.
4. **Dependency direction.** A script step asserts `cargo tree -p universe-core -e normal --locked` contains no `bevy`, `wgpu`, or `winit`, and `cargo tree -p universe-render -e normal --locked` contains no `universe-app`. If `deny.toml` is adopted (open question 3), `[bans]` with `wrappers = ["universe-render", "universe-app"]` on `bevy` does the same with advisories and licenses as a bonus.
5. **Doc index guard.** Extends the existing doc drift guard: every file and folder directly under `docs/` is named in `docs/README.md`; every `docs/decisions/*.md` matches `NNNN-[a-z0-9-]+.md` with unique, consecutive numbers; `CLAUDE.md` contains no rule sentences (only pointers).
6. **File size guard (soft).** The doc drift guard prints a warning for any `.rs` file over 800 lines and fails over 1,200. Thresholds live in `docs/engineering.md` and the script reads them from `.agent/project-config.json` (same pattern as status IDs).
7. **Feature matrix (dormant until the first feature exists).** The day a crate defines its first feature, CI adds `cargo hack check --each-feature --workspace --locked` (or, at minimum, `check` and `test` with `--no-default-features` and with `--all-features`) so every feature combination keeps compiling. Documented now so it is not forgotten then.

### 4.7 Testing conventions

Testing ladder, cheapest first (from the Bevy Book), written into `docs/engineering.md`:

1. Pure function or type test, no ECS (`universe-core`; the default).
2. Raw `World` for setup helpers.
3. `World::run_system_once` for one Bevy system.
4. `Schedule::run` when system order matters.
5. `App` with `MinimalPlugins` plus the plugin under test, stepped with `app.update()`, with an iteration cap.

Rules: no test opens a window or needs a GPU; the headless journey replay (`replay_autopilot`) is the integration test of record and lives in core; `--verify` remains the end-to-end gate and its output is treated as a golden file during the migration (section 5, M0); every bug fix adds a test at the lowest rung that reproduces it.

### 4.8 Cargo features policy

Cargo features are good practice for what they were designed for and a poor substitute for crate boundaries. The compiler checks a crate boundary on every build; it checks a feature-gated boundary only when that feature is on.

- **Consuming features.** Every dependency is declared `default-features = false` with an explicit feature list, and each enabled feature has a written reason (Bevy already follows this in the root `Cargo.toml` and `docs/universe/stack.md`). The same rule applies to any future dependency.
- **Defining features.** A crate defines a feature only for one of three reasons: an optional dependency, a platform or backend choice, or developer-only tooling (a `devtools` feature, on by default, turned off for release builds, as the Bevy Book recommends). A feature is never used for a runtime choice (`--verify` and `--capture` stay CLI flags), for architectural separation (that is a crate boundary), or for product variants.
- **Additive only.** Enabling a feature never removes or changes behavior that exists without it. Every combination must compile and pass tests.
- **Declared in three places.** A one-line comment next to the feature in `Cargo.toml`, a line in the crate-level doc comment, and a row in `docs/engineering.md`. The `default` feature is always written out, even when empty.
- **Today: none.** No crate defines a feature and none is needed for this migration. The Bevy Book's `headless` feature is a workaround for single-crate projects; this project is headless by construction because `universe-core` has no Bevy dependency. The first plausible candidate is `devtools` in `universe-render`, when a debug overlay or diagnostics system appears.

### 4.9 Error handling, panics, and diagnostics

State today: non-test code contains exactly one `.expect` (`astro.rs`, converting a grown `Vec` of eight octants into an array, a proven invariant); every other `unwrap`/`expect` is inside `#[cfg(test)]`. Core signals failure with `Option` only (`Level::new`, `Constraints::new`, `Frame::new`) and defines no error types. The binary prints a machine-readable line protocol to stdout and exits 0 or 1. No code uses `tracing`/`bevy_log` macros even though `bevy_log` is enabled. The validator script scans every line of stdout and stderr and treats matches of `error`, `panic`, `warn`, `deprecat`, `FAILED`, or `VERIFY-FAIL` as failures or warnings. Those last two facts make output vocabulary an interface, not a detail.

Rules (sources: Rust API Guidelines, `anyhow` and `thiserror` documentation on the library/application split, Clippy `unwrap_used` / `expect_used` documentation, Bevy `bevy::ecs::error` documentation on fallible systems, Bevy `LogPlugin` documentation):

- **No panics in library code.** `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, and indexing that can go out of bounds are forbidden outside tests. **CI-checked**: `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented`, `clippy::indexing_slicing` set to `deny`, with `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` in `clippy.toml`. The one existing `expect` is rewritten with `try_into().map_err(...)` or `#[expect(clippy::expect_used, reason = "...")]`, which fails the build the day the reason stops being true.
- **`Option` versus `Result`.** `Option` when absence is a normal outcome with one obvious meaning ("no marker under the cursor", "L15 has no child"). `Result<T, E>` with a named error type when the caller needs the reason or must report it (invalid constraints, unreadable capture directory). Converting `Option` to a silent early return is allowed only when absence is normal; a missing resource or window that indicates a bug must surface (next rule).
- **Error types.** `universe-core` stays std-only: its error types are small `enum`s with hand-written `Display` and `std::error::Error` impls, one per module that needs one, named `<Module>Error`. `universe-render` may reuse them. `universe-app` is the only crate allowed a type-erased error (`Box<dyn Error>` or, if the owner approves the dependency, `anyhow`). `thiserror` is adopted only if a third hand-written impl appears; until then it is not worth a dependency.
- **Fallible Bevy systems.** Bevy 0.19 lets a system or observer return `Result<(), BevyError>`; the app chooses the handler with `App::set_error_handler`. Rule: a system that can fail for a reason that is a bug returns `Result` and uses `?`; the app installs `panic` in debug builds and `error` in release builds so development stops on the bug and a shipped build logs it. Systems that early-return on normal absence (no cursor position, no window yet) keep `let ... else { return }`.
- **Exit codes and the stdout protocol.** stdout is reserved for the machine protocol consumed by the validator and by `--capture`: `VERIFY-OK` / `VERIFY-FAIL`, `CAPTURE-OK` / `CAPTURE-FAIL`, and the `LEVEL`, `SNAPSHOT`, `JOURNEY`, `RATIO`, `PREVIEW` lines. The exact line formats are documented in `docs/engineering.md` (section "Headless protocol") and changing one is a decision (ADR) because `scripts/validation/validate.py` and the golden files depend on them. Exit code 0 means every check passed, 1 means a check failed, 2 means usage error (bad flags); the current code conflates the last two and is corrected in M5.
- **Diagnostics go to `tracing`, never to `println!`.** Human-facing messages use the `bevy::log` macros (`info!`, `warn!`, `error!`, and the `*_once!` variants inside per-frame systems so a condition that holds every frame logs once). The default filter is `warn,universe_core=info,universe_render=info,universe_app=info`; `RUST_LOG` overrides it (Bevy's `LogPlugin` behavior). Structured fields (`level = %level, marker = marker`) instead of formatted strings, so logs stay greppable.
- **Vocabulary is an interface.** Because the validator fails on the words `error`, `panic`, `warn`, `deprecat`, and `FAILED` anywhere in the output, those words are reserved for real problems. An informational line never contains them (write "0 mismatches", not "0 errors"); `warn!` and `error!` are used only when a human should act. **CI-checked** by the existing validator run; stated here so no one "fixes" a false positive by weakening the regex.
- **Backtraces in automated runs.** CI and the validator run with `RUST_BACKTRACE=1` so a panic in `--verify` is diagnosable from the log alone.

### 4.10 Performance, determinism, and build speed

State today: no `[profile.*]` section and no `.cargo/config.toml`, so dev builds run Bevy unoptimized and release builds use Cargo defaults. `universe-core` calls `powf`, `ln`, `sin`, `cos`, and `asin`, which Rust forwards to the platform C math library (glibc on Linux, UCRT on Windows); results can differ in the last bit between platforms. Snapshots print six fractional digits (`SNAPSHOT_PRECISION`), which hides most but not all such differences. CI runs `--verify` on Linux and Windows but never compares the two outputs, so determinism is proven per machine, not across platforms. `PreviewCache::sync` allocates `path.indices()` and `preview_set` every frame, against the stated "no per-frame allocation in hot systems" rule.

Rules (sources: Bevy Book "Fast Compiles" and "Compiling Release Builds", Bevy `docs/profiling.md`, Rust `f64` documentation on unspecified precision of transcendental functions, Cargo profiles reference):

- **Determinism is a contract with three tiers**, written in `docs/ARCHITECTURE.md`: (1) same binary, same machine: byte-identical, **CI-checked** today by the repeat run; (2) same source, different platform: byte-identical snapshots at six digits, **CI-checked** from M0 on because the golden test (section 4.11) runs against the same committed file on the Linux and the Windows job; (3) different compiler version: not promised. Transcendental calls (`sin`, `cos`, `exp`, `ln`, `powf`, `asin`) are allowed only in `universe-core` generators and listed in one place (`docs/engineering.md`, "Numerics") so a future move to a pure-Rust `libm` or to integer noise is a one-ADR change. Integer and bit-exact operations (`+ - * / sqrt`, `total_cmp`, `rem_euclid`, hashing) are preferred wherever the result feeds a seed or a sort.
- **Never `-C target-cpu=native`** or any flag that changes floating-point codegen between machines; `mul_add` is used only when the fused result is wanted and documented.
- **Floats never cross a sort or a hash un-canonicalized.** `total_cmp` for ordering (already the case in snapshots), explicit rounding to `SNAPSHOT_PRECISION` before any text comparison, and no `f32` in `universe-core` positions (the floating-origin design keeps `f64` in core and converts to `f32` at the render boundary only).
- **Profiles in the root `Cargo.toml`.** `[profile.dev] opt-level = 1`, `[profile.dev.package."*"] opt-level = 3` (Bevy's recommendation: the game stays debuggable while the engine runs at speed); `[profile.release] lto = "thin", codegen-units = 1, strip = "debuginfo"`; a `[profile.profiling]` inheriting release with `debug = 1` for flamegraphs. Committed, so every machine and CI build the same way.
- **Build speed is measured, not guessed.** `cargo build --timings` before and after any crate split (M3 to M5) and the two numbers posted on the sub-issue. `bevy/dynamic_linking` and alternative linkers (`lld`, `mold`) are developer-local choices documented in `docs/engineering.md`, never committed in `.cargo/config.toml`, because they change the artifact.
- **Hot-path rules, each with its probe.** No allocation per frame in `Update` systems (probe: `PreviewCache::regenerations` style counters plus, when needed, `bevy/trace_tracy_memory`); gizmo drawing stays immediate-mode and bounded by the live-generation cap (`2 + PREVIEW_CAP`); any `Vec` built every frame is replaced by a reused buffer or a fixed-size array (no new dependency for this). The existing `sync` allocation is fixed in M3 when the code moves, with a test that asserts zero regenerations and zero re-allocations for a static camera.
- **Profiling is a dev-only feature, which is the one legitimate feature of section 4.8.** `universe-app` may define `profiling = ["bevy/trace_tracy"]` (off by default) so `cargo run --release --features profiling` streams spans to Tracy; `cargo flamegraph` with the `profiling` profile covers sampling. Both procedures live in `docs/engineering.md`.
- **Budgets are owner decisions and stay in `docs/universe/stack.md`** (60 fps, draw-call and texture budgets TBD). `docs/engineering.md` only says how each is measured: frame time from Bevy's `FrameTimeDiagnosticsPlugin` behind `profiling`, generation time printed by `--verify` as a `TIMING` line (wall-clock per `generate_cell`, informational, never a pass/fail criterion because CI machines vary).

### 4.11 Testing techniques

State today: 72 tests, all inline unit tests in `universe-core` (64) and `universe-render` (8); none in `universe-app`. The `--verify` and `--capture` outputs are checked only by `scripts/validation/validate.py`, never by `cargo test`, so a developer running the test suite cannot see a protocol regression. Determinism is asserted on three hand-picked seeds. No property-based tests, no golden files, no `#[ignore]`, no `should_panic`. Test names already state the property in plain words (`dive_step_closes_and_never_crosses_surface`), which section 4.7 keeps.

Rules (sources: `insta` and `expect-test` documentation on the snapshot review workflow, `proptest` book on properties versus examples and on persisted regressions, `cargo-nextest` and `cargo-llvm-cov` documentation, rust-analyzer `architecture.md` on data-driven tests):

- **Golden files for the headless protocol.** `crates/universe-app/tests/golden.rs` runs the built binary (`env!("CARGO_BIN_EXE_universe-app")`) with `--verify` and compares stdout byte-for-byte with the committed `crates/universe-app/tests/golden/verify.txt`; a second test runs `--capture` into a temporary directory and compares a sorted listing of file names plus a hash per file with `tests/golden/capture.txt`. Setting `UPDATE_GOLDEN=1` rewrites the files (the review-then-commit workflow of `insta`, without the dependency: the diff in the pull request is the review). CI never sets the variable, so an unreviewed change fails. The golden files are the M0 baseline made permanent.
- **One printer.** Every snapshot and every golden comparison goes through `snapshot_generated` and the `protocol` module; no test formats floats on its own. The project already has its canonical text format, so `insta` is not adopted.
- **Properties, not only examples.** `proptest` becomes a dev-dependency of `universe-core` (dev-dependencies do not ship and do not count against "std only"; `docs/engineering.md` says so explicitly). First properties, each one a sentence in the test name: for any seed and any marker chain, generating twice gives identical snapshots; for any chain, `open` then `close` returns the same offset within `1e-12`; for any factor in `(0, 1]`, `dive_step` never crosses the surface; `snapshot_generated` is invariant under permutation of the input points; `check_border(a, b)` equals `check_border(b, a)`. The `proptest-regressions/` directories are committed, so a found counter-example is replayed forever.
- **Bevy-side tests exist.** Today no test builds an `App`. M4 adds, in `universe-render`: a test that builds `App::new()` with the minimal plugin set `DivePlugin` needs, runs `app.update()` three times, and asserts no panic and that `Universe` is at `Level::MIN`; and the schedule-order test named in M4. The plugin set is documented next to the test so the "headless by construction" claim has a probe.
- **Doc tests teach, unit tests check.** A public function whose contract is not obvious from its signature (`dive_step`, `path_seed`, `preview_set`) carries one doctest example; everything else is a unit test. `cargo test --doc` already runs in CI.
- **Tests are hermetic.** No wall clock, no randomness without a fixed seed, no files outside a per-test temporary directory (`std::env::temp_dir()` plus the test name; the `tempfile` crate only if cleanup becomes a problem), no network. A test that needs the GPU does not exist.
- **Runner and coverage, by need.** `cargo test` remains the runner while the suite finishes in under a minute; `cargo nextest` is adopted when flakiness detection, per-test timeouts, or run time justify one more CI binary (doctests then stay on `cargo test --doc`). `cargo llvm-cov` runs on pull requests as information in the job summary, never as a gate: a coverage threshold rewards tests that execute code without asserting anything.
- **Every bug fix adds the failing test first**, at the lowest rung of the section 4.7 ladder that reproduces it, named after the symptom; correction rounds under `docs/workflow.md` cite that test on the Issue.

### 4.12 API design and idioms

State today: `Level(u8)` is a validated newtype with `new -> Option`, `MIN`/`MAX`, `deeper`/`shallower`; it is the model to generalize. Positions travel as bare `[f64; 3]` in two unit frames (open-cell units and parent units; `sibling_in_open_units` converts between them), seeds as bare `u64`, marker indices as bare `u32`. 109 `pub fn` versus 3 `pub(crate)`; 60 numeric `as` casts; no `#[must_use]`, no `Display`, no `From`/`TryFrom` impls, no `SystemSet`. The Bevy 0.19 split between buffered `Message`s (`MessageReader<MouseWheel>`, `MessageWriter<AppExit>`) and observed `Event`s is already used correctly.

Rules (sources: Rust API Guidelines checklist, Bevy Book on visibility and plugin organization, Bevy 0.17 to 0.19 release notes and migration guides on Messages versus Events and the `Systems` suffix, Clippy cast lints):

- **Visibility is `pub(crate)` by default.** `pub` is for items another crate uses; each crate root documents its public surface. **CI-checked** with the rustc lint `unreachable_pub = "warn"` in `[workspace.lints.rust]`, which flags `pub` items not reachable from the crate root, so the choice is explicit at every item.
- **Newtypes for anything that has units or an invariant (C-NEWTYPE).** Pattern: `Level`. New code introduces `Seed(u64)`, `MarkerIndex(u32)`, and unit-tagged positions (`OpenUnits([f64; 3])`, `ParentUnits([f64; 3])`, or a single `Offset<Frame>` with a zero-sized frame marker) so the compiler, not a comment, keeps parent and child units apart. Conversion between frames is a named method (`to_open_units(entered: Opened)`), never arithmetic at the call site. Existing bare arrays are migrated in M3 as a types-only change, proven by the golden tests; the migration stops at the render boundary where Bevy's `Vec3` takes over.
- **Numeric casts are deliberate.** `f64::from`, `u32::from`, `usize::from` where lossless; `TryFrom` where a range must hold; `as` only in one place per lossy conversion with `#[expect(clippy::cast_..., reason = "...")]`. The `f64 -> f32` boundary already lives in `to_vec3`; it stays the only such site in `universe-render`. **CI-checked** with `clippy::cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`, `cast_possible_wrap`, and `cast_lossless` at `warn` (fails CI under `-D warnings`), reviewed once in M2 over the 60 current sites.
- **Common traits eagerly (C-COMMON-TRAITS).** Every public type derives `Debug` and `Clone`; `Copy` when it is small and plain; `PartialEq`, and `Eq` + `Hash` when it contains no floats; `Default` when a neutral value exists; `PartialOrd`/`Ord` only when the order means something (`Level`). `Display` is implemented only for user-facing text (the label types from `labels`), never as a second `Debug`.
- **`#[must_use]` on pure functions** whose result is the whole point (`dive_step`, `should_open`, `preview_set`, `path_seed`), so an ignored result is a compile warning. **CI-checked** with `clippy::must_use_candidate` at `warn`.
- **Constructors validate (C-VALIDATE); conversions use the standard traits (C-CONV-TRAITS).** `new` returns `Option` or `Result` when an input can be invalid (already the case for `Level`, `Constraints`, `Frame`); `From`/`TryFrom` instead of ad-hoc `to_x` methods when the conversion is total or checked; method names follow `as_` (free, borrowed), `to_` (costly), `into_` (consuming).
- **Fields match their struct (Bevy Book).** Plain-data types keep public fields (`Generated`, `JourneyStep`); types with an invariant (`MarkerPath`, `PreviewCache`, `Level`) keep private fields and named methods. No getter/setter pairs that merely forward.
- **Enums over booleans and `Option` arguments (C-CUSTOM-TYPE).** `Autopilot::{Idle, Flying}` is the pattern; a new `bool` parameter that is not obviously a predicate becomes a two-variant enum.
- **Documentation sections.** A function that returns `Result` has an `# Errors` section; one that can panic has `# Panics` (after section 4.9 there are none outside tests). **CI-checked** with `clippy::missing_errors_doc` and `missing_panics_doc` at `warn`.
- **Bevy naming and mechanisms.** Plugins end in `Plugin`; system sets end in `Systems` (the Bevy 0.17 convention: `DiveSystems::{Input, Camera, Draw}`); components and resources carry no suffix; marker components are unit structs. Input and anything "many per frame, read later" is a `Message`; a state change other plugins react to immediately is an `Event` with an observer; a type never implements both. Spawned entities carry a `Name` (the camera today has none) so inspectors and logs can say which entity. Prefer the `Single<&Window>` system parameter, which skips the system when there is not exactly one, over `Query<&Window>` plus a manual `single()` early return.
- **No `unsafe`.** `unsafe_code = "forbid"` in `[workspace.lints.rust]` (section 4.6); lifting it is a decision (ADR), and the only acceptable reason would be a measured hot-path need that safe code cannot meet.

### 4.13 Dependencies, supply chain, and CI hygiene

State today: `Cargo.lock` resolves 423 packages and nothing scans them for advisories or license terms. CI actions are pinned by major tag (`actions/checkout@v4`, `dtolnay/rust-toolchain@v1`), not by commit SHA; the workflow has no `permissions:` block and no `concurrency` group; there is no Rust build cache, so both jobs compile Bevy from scratch on every push. No Dependabot or Renovate configuration. No `rust-version`, `license`, or `publish = false` in any manifest; no `LICENSE`, `.editorconfig`, or `typos` configuration at the root. Existing strengths: exact pins for the toolchain and Bevy, `Cargo.lock` committed, every CI command `--locked`.

Rules (sources: Cargo reference on `rust-version` and profiles, `cargo-deny` and `cargo-audit` documentation, RustSec guidance, GitHub Actions security hardening guidance on SHA pinning and token permissions, `Swatinem/rust-cache` documentation, Microsoft Rust engineering practices on supply chain):

- **Three kinds of version, three policies.** Toolchain and Bevy stay exact pins, changed only by an ADR (already the rule). Direct dependencies other than Bevy use caret requirements in `[workspace.dependencies]` and are pinned by `Cargo.lock`. Transitive dependencies are owned by the lockfile. `cargo update` is never run casually: a lockfile change is its own pull request with the `cargo tree` diff in the body.
- **Advisories and licenses are checked, not assumed.** If open question 3 is yes, `deny.toml` covers advisories (RustSec, including yanked and unmaintained), licenses (allow list, `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, extended only by ADR), bans (`bevy` wrapped by `universe-render` and `universe-app`; `multiple-versions = "warn"`), and sources (crates.io only). If no, `cargo audit` runs as the minimum. Either runs on every pull request and on a weekly schedule, because advisories appear without any commit.
- **Updates arrive as pull requests, not surprises.** `.github/dependabot.yml` with two ecosystems: `github-actions` (weekly) and `cargo` (weekly, lockfile-only, grouped into one pull request, with Bevy and anything `=`-pinned ignored so the exact pins are never touched by a bot). Each such pull request is a `chore/` branch with "no parent Issue" and goes through CI like any other; the golden tests (section 4.11) are what make a dependency bump safe to merge without a validator round.
- **Manifests say what they are.** `[workspace.package]` gains `rust-version = "1.97.1"` (matches the toolchain pin; Cargo refuses older compilers with a clear message and Clippy's `incompatible_msrv` lint can use it), `license` (open question 9), `repository`, and `publish = false` inherited by every crate so a stray `cargo publish` cannot happen. The redundant `[lib]`/`[[bin]]` tables are dropped (M2).
- **CI is hardened and cached.** Every third-party action is pinned to a full commit SHA with the version in a trailing comment (Dependabot keeps them current). The workflow declares `permissions: contents: read` at the top and widens per job only where a step needs more (the parent auto-close guard reads issues through the API and needs `issues: read`). A `concurrency` group keyed on the workflow and branch cancels superseded runs. `Swatinem/rust-cache` runs after the toolchain step on both jobs; the cache key already includes `Cargo.lock` and `rust-toolchain.toml`, so a pin change starts a fresh cache. `cargo build --timings` before and after is posted on the M2 sub-issue as the evidence that the cache pays for itself.
- **Release profile is conservative.** `lto = "thin"`, `codegen-units = 1`, `strip = "debuginfo"` (section 4.10); `panic = "unwind"` stays so the error handler and panic hook can log before exit. `panic = "abort"` is an option recorded in `docs/engineering.md` for a future size-sensitive build, not a default.
- **Small hygiene tools, each with a reason.** `.editorconfig` (UTF-8, LF, final newline, no trailing whitespace, four-space indent for Rust and Markdown) so editors prevent what the Markdown lint rejects. `typos` (`typos.toml` at the root, one CI step) because this repository is Markdown-heavy and the owner reads the documents. `cargo-machete` is not added until a crate has more than a handful of dependencies; `cargo-vet` is not adopted: the project has no external consumers that require audited trust, and `cargo-deny` covers the risk at this size. No `CHANGELOG.md`: the Issues and the Done column are the record under `docs/workflow.md`, and a second log would drift.
- **Commit and branch conventions are already practiced and only written down.** Branch names follow the CI guard (`feat/<issue>-<slug>`, `fix/<issue>-<slug>`, `chore/<slug>`). Commit subjects are imperative, under 72 characters, and name the Issue when one exists (`Correction R6 for #35: nested dive`); squash merges keep the pull request title as the subject. A `CONTRIBUTING.md` of five lines points newcomers at `AGENTS.md`, `docs/engineering.md`, and `docs/ARCHITECTURE.md`; it carries no rules of its own, so the doc drift guard treats it like `CLAUDE.md`.

## 5. Migration needs

Ordered. Each step is one sub-issue, lands on the same feature branch or as consecutive PRs on the parent Issue, and is **behavior-preserving**. The proof of "unchanged" is the same for every step and is a test, not a manual diff: the golden tests from M0 pass (`--verify` stdout and `--capture` files byte-identical to the committed baseline), `cargo test --locked --workspace` passes with the same test names plus the ones each step adds, and the owner's manual check of the window finds nothing different.

### M0. Baseline as tests

- Add `crates/universe-app/tests/golden.rs` and the two golden files (section 4.11) generated from current `main`, before any other change. This is test-only and changes no behavior.
- Run the golden test on both CI platforms. If Linux and Windows disagree, that is the tier-2 determinism finding from section 4.10, surfaced on day one: record both outputs on the Issue, keep the Linux file as the committed golden, mark the Windows job's golden test as informational until open question 8 is answered.
- Record `cargo test --workspace -- --list` on the Issue so the test inventory can be compared at M7.

### M1. Documents and instruction files

- Create `docs/README.md` (index), `docs/ARCHITECTURE.md` (codemap of the **current** layout, including the known violation in `universe-render`, so the document is truthful from day one), `docs/engineering.md` (sections 4.2 to 4.7 of this draft, each rule tagged CI-checked or convention).
- Create `docs/decisions/` with back-filled ADRs, each citing its Issue: 0001 use MADR; 0002 Rust stable pinned exactly, edition 2024 (#35); 0003 Bevy `=0.19.1`, minimal features, indicators only (#35 spec v4); 0004 three-crate layering, headless core (#35); 0005 fixed timestep and 60 fps target (owner decisions on #35); 0006 marker-tree nesting (R6, #58/#61); 0007 pre-entry preview (R7, #63); 0008 headless `--verify` as the automated acceptance gate.
- Move `CLAUDE.md` rules into `docs/engineering.md`; reduce `CLAUDE.md` to pointers; add the References line and the always-on block to `AGENTS.md` (section 4.5).
- Assign an `E-` ID to every rule; write the "References" appendix from section 3 and the per-section sources; write ADR 0009 with section 9 of this draft as its context.
- Post the traceability table on the Issue: one row per rule in sections 4.2 to 4.13 with its landing file and ID. The sub-issue is not done while a row is empty.
- Skill updates: `implementation` (read `docs/engineering.md`, run the CI-checked list locally before the pull request), `technical-validation` (rule-compliance line in the report), `planning` (cite the `E-` IDs a plan relies on). `.github/PULL_REQUEST_TEMPLATE.md` gains the "rule added, changed, or retired?" question.
- Extend the CI doc drift guard with the section-list check, the `E-` ID existence check, the pointer-only check for `CLAUDE.md` and `CONTRIBUTING.md`, the doc index check, and the ADR naming check (section 4.5 and section 4.6, item 5).
- Move the "Workspace crates and dependency rules" and "Architecture rules" sections of `docs/universe/stack.md` to the new documents and leave a pointer (pending open question 6).

### M2. Tooling, lint policy, and CI hygiene

- Add `rustfmt.toml`, `clippy.toml` (including `allow-unwrap-in-tests`, `allow-expect-in-tests`), `[workspace.lints]` (including the panic-family lints from section 4.9), and `[lints] workspace = true` in all three crates.
- Add the `cargo doc` step and the dependency-direction step to CI. Add `deny.toml` and a `cargo-deny` step only if the owner says yes (open question 3).
- Add the `[profile.*]` sections from section 4.10 to the root `Cargo.toml`. Optimization level must not change `--verify` output; the M0 golden test proves it on both platforms. If it fails only after the profile change, the difference is posted on the sub-issue and open question 8 is answered before continuing.
- Add `proptest` as a dev-dependency of `universe-core` with the five properties from section 4.11, and commit the `proptest-regressions/` directory (empty at first).
- Fix whatever the new lints report. No logic changes; if a lint cannot be satisfied without changing behavior, `#[allow]` it with a one-line reason and note it on the sub-issue.
- Remove the stale dependency comment in `crates/universe-render/Cargo.toml`; drop the redundant `[lib] name/path` tables (Cargo infers them) unless a reason is found.
- Manifest metadata from section 4.13: `rust-version`, `license` (after open question 9), `repository`, `publish = false` in `[workspace.package]`, inherited by every crate.
- CI hardening from section 4.13: SHA-pinned actions, top-level `permissions: contents: read` with per-job widening, a `concurrency` group, `Swatinem/rust-cache` on both jobs, a weekly scheduled run of the advisory check, and `typos`. `.github/dependabot.yml` for `github-actions` and lockfile-only `cargo` updates with the exact pins ignored. `.editorconfig` and the five-line `CONTRIBUTING.md`. Build-time before and after caching posted on the sub-issue.

### M3. Move pure logic from `universe-render` to `universe-core`

- New module `universe_core::nav` (or `journey`; name decided in planning) receives: `Universe` (as a plain struct; `universe-render` wraps it in a `#[derive(Resource)]` newtype or keeps a thin resource that holds it), `DiveEvent`, `dive_step`, `should_open`, `should_close`, `sibling_in_open_units`, `drawn_radius`, `open_marker_radius`, `PreviewCache`, `preview_positions`, `preview_error`, `JourneyStep`, `replay_autopilot`, and the constants `DEMO_SEED`, `MIN_NAV_LEVEL`, `MAX_NAV_LEVEL`, `START_OFFSET`, `MIN_MARKER_ANGLE`, `WHEEL_FACTOR`, `KEY_RATE`, `AUTOPILOT_RATE`, `ROOT_MAX_DISTANCE`.
- New module `universe_core::labels` receives `level_label`, `scale_label`, `scale_anchor`, `window_title_for_level`.
- Types-only hardening from section 4.12 on the moved API: `Seed`, `MarkerIndex`, and unit-tagged positions replace the bare `u64`, `u32`, and `[f64; 3]` parameters of the `nav` module; `#[must_use]` on its pure functions; `unreachable_pub` findings resolved by making items `pub(crate)` or re-exporting them deliberately. Planning may defer the position newtypes to a follow-up sub-issue if the diff grows past the step's review budget; the rule still applies to new code from M3 on.
- The tests that moved with the code keep their names. `universe-render` re-exports the moved items during the step so `universe-app` compiles unchanged; the re-exports are removed in M5.
- After this step the dependency-direction invariant "everything `--verify` needs is reachable from core alone" becomes CI-checkable.

### M4. Split `universe-render` into modules

- `lib.rs` keeps the crate doc, the two public plugins, and `pub use` of the few helpers the app needs.
- `camera.rs` (`spawn_indicator_camera`, `sync_camera`), `input.rs` (`handle_quit`, `pick_hover`, `handle_input`, `Navigation`, `Autopilot`), `draw.rs` (`draw_axes`, `draw_open_cell`, `draw_previews`, `draw_parent_siblings`, `sync_previews`), `style.rs` (`point_color_for_level`, `sibling_color_for_level`, `scaled`, `to_vec3`, `PICK_PIXELS`).
- `DivePlugin` stays the public entry point. Internally it declares a `DiveSystems` set (variants `Input`, `Camera`, `Draw`, following the Bevy `Systems` suffix convention) chained in today's order: quit, hover, input, previews, camera, draw open cell, draw previews, draw siblings. The order is written once and asserted by a test that reads the schedule graph, so a future module split cannot silently reorder it.
- The camera entity gains a `Name("indicator camera")`; window access in `pick_hover` and `sync_camera` moves to the `Single<&Window>` parameter (section 4.12). Both are behavior-neutral and covered by the new `App`-level test from section 4.11.

### M5. Split `universe-app`

- `main.rs` keeps flag parsing and the window `App`; `verify.rs` receives `run_verify`, `verify_preview`, `verify_ratios`, `verify_inverse`, `milestone_tag`, `flag`; `capture.rs` receives `run_capture` and its helpers; `protocol.rs` owns every stdout line format from section 4.9 so the protocol is written in one place.
- Usage errors (missing `<dir>` after `--capture`, unknown flag) exit with code 2 instead of 1; `--verify` and `--capture` keep 0/1. `scripts/validation/validate.py` is checked for any dependency on the old code and the golden files from M0 are compared on stdout content, which this does not change.
- Install the Bevy error handler (`panic` in debug, `error` in release) when building the window `App`; convert any system whose early return hides a bug into a `Result`-returning system. The `--verify` output is unaffected because no system runs headless.
- Imports switch from `universe_render::*` re-exports to `universe_core::nav` and `universe_core::labels`; the temporary re-exports from M3 are removed.
- Optional (open question 4): a fourth crate `universe-verify` (binary, depends on `universe-core` only) so the headless gate never links Bevy and CI can run it in seconds. If adopted, `universe-app --verify` delegates to it or is removed in a later owner-approved change; not part of this migration.

### M6. Hygiene

- `.gitignore`: ignore `tmp/` entirely except `tmp/**/README.md`; keep the existing validation and `__pycache__` rules.
- `scripts/README.md`: one paragraph per subfolder (`backfill`, `gates`, `intake`, `sidebar`, `validation`) and the shared rule "IDs come from `.agent/project-config.json`".
- `.agent/README.md`: what `project-config.json`, `backfill/`, and `validation/` hold and what is tracked versus untracked.
- Remove the stale `tmp/capture-spot-*` directories from the working tree (they are not tracked).

### M7. Close the loop

- Update `docs/ARCHITECTURE.md` to the final shape and remove the "known violation" note from M1.
- Record ADR 0009 "repository organization and enforcement" pointing at this Issue.
- Confirm the golden tests still pass unchanged (the golden files were not rewritten at any step; `git log` on `tests/golden/` shows only the M0 commit) and compare the test inventory with the M0 list on the Issue.

## 6. Non-goals

- No change in behavior, visuals, controls, timings, or `--verify` / `--capture` output.
- No Bevy or Rust upgrade; pins stay exactly as they are.
- No new universe features; L12 to L15 remain out of scope.
- No `docs/universe/ladder.md` change (frozen).
- No mdBook site, no `xtask`, no `examples/` or `benches/` folders, no `cargo nextest`, no `cargo-vet`, no `CHANGELOG.md`, no `thiserror`/`anyhow`; each is listed in `docs/engineering.md` as "adopt when needed" with the condition that would trigger it.
- No change to the process documents (`docs/workflow.md`, `docs/board.md`, `docs/validator-capabilities.md`) or to the skills, other than the single reference line added to `AGENTS.md`. The validator's output regex is not weakened; section 4.9 adapts the code's vocabulary to it instead.
- No license choice by the AI (open question 9) and no change to the exact Bevy and toolchain pins by any bot; Dependabot is configured to ignore them.

## 7. What the owner will be able to check

- Open the window and use it exactly as before: hover highlights, click targets, scroll and arrows dive, Spacebar runs the autopilot to the planet, Esc quits. Nothing looks or feels different.
- Open `docs/README.md` and find, in one click each, the architecture map, the technical rules, and the list of decisions.
- Open `docs/ARCHITECTURE.md` and answer "where is the code that decides when a marker opens?" in under a minute (answer: `universe-core`, `nav` module, `should_open`).
- See that the AI's validator confirms: `--verify` output identical to the baseline, CI green including the new checks, and `universe-core` free of any Bevy dependency.
- Open `docs/engineering.md` and see, for any rule, whether a machine checks it (tagged CI-checked, with the check named) or a reviewer does (tagged convention), and find at the end the list of sources that were read to write it.
- Start a brand-new AI session with no history and ask "what are this project's technical rules and where do they come from?"; the answer should come from `AGENTS.md`, `docs/engineering.md`, and `docs/decisions/` alone, and should cover everything in this draft. That is the test that the memory is in the repository, not in a conversation.
- Notice that pull requests finish CI faster (build cache) and that, once a week, a bot may open a small pull request updating the lockfile or an action; the Bevy and Rust versions never change that way.

## 8. Open questions for the owner

1. **Home of the technical instructions.** Recommended: `docs/engineering.md`, with `CLAUDE.md` reduced to a pointer. Alternatives: keep everything in `CLAUDE.md`; or use a root `CONTRIBUTING.md`.
2. **Lint strictness.** Recommended: clippy default groups plus a short cherry-picked `pedantic` list (for example `missing_errors_doc`, `needless_pass_by_value`, `semicolon_if_nothing_returned`). Alternative: default groups only.
3. **`cargo-deny`.** Recommended yes: it adds advisories, licenses, duplicate-version checks, and the layer ban in one tool. Cost: one more binary installed in CI. Alternative: the `cargo tree` script only.
4. **Separate `universe-verify` binary without Bevy.** Recommended as a follow-up Issue, not in this migration.
5. **One parent Issue or two.** Recommended one parent Issue with M0 to M7 as sub-issues, because every step shares the same acceptance proof. Alternative: documents and tooling (M1, M2, M6) as one Issue, code reshaping (M3 to M5, M7) as another.
6. **`docs/universe/stack.md` sections.** The crate and architecture rule sections came from spec v4 of #35. Moving them to `docs/ARCHITECTURE.md` and `docs/engineering.md` (leaving a pointer) needs the owner's approval because that file records approved spec content.
7. **Usage-error exit code.** Section 4.9 proposes exit code 2 for bad command-line usage (today it is 1, the same as a failed check). This is the only behavior change in the draft and it affects no normal run; it needs a yes or no.
8. **Cross-platform numerics, only if the M0 golden test shows a platform difference.** Options: (a) accept per-platform determinism and record it as tier 1 only; (b) depend on the pure-Rust `libm` crate in `universe-core` for transcendental functions (one small `no_std` dependency, identical results everywhere, a documented exception to "std only"); (c) rewrite the generators to integer noise. Recommended: measure first; if a difference appears, option (b) as the smallest change that keeps the notion's "same universe every time" true on every machine.
9. **License.** The crates carry no `license` field and the repository has no `LICENSE` file. The owner chooses (for a private or not-yet-published project the usual choices are `MIT OR Apache-2.0`, or an explicit "all rights reserved" with `publish = false`); the AI does not pick a license. Needed before `cargo-deny`'s license check can run on the workspace's own crates.

## 9. Revision notes

Each loop of search, review, investigate, update, note, refactor is recorded here so the owner can see what changed and why.

### Loop 1 (2026-10-02): error handling, panics, diagnostics

- Searched: Bevy 0.16+ fallible systems and `App::set_error_handler`; `anyhow`/`thiserror` library-versus-application split; Clippy `unwrap_used`/`expect_used` and their `clippy.toml` test allowances; Bevy `LogPlugin` and `tracing` filtering.
- Investigated in the repository: all `unwrap`/`expect`/`panic` sites (one in non-test code), all output sites (stdout protocol only, no `tracing` use), and how `scripts/validation/validate.py` scans output (regex on `error|panic|warn|deprecat|FAILED`).
- Added section 4.9 with eight rules; marked which are CI-checked. Added `protocol.rs` and the error-handler install to M5; added the panic-family lints to M2.
- Refactor of the draft: open question 7 records the single behavior change (exit code 2) so section 1's "no behavior change" promise stays honest.

### Loop 2 (2026-10-02): performance, determinism, build speed

- Searched: Bevy fast-compile setup (dev profile, dependency `opt-level`, `dynamic_linking`, `lld`/`mold`), Bevy release-build profile guidance, Bevy profiling (`trace_tracy`, `cargo flamegraph`), Rust floating-point determinism (platform `libm`, FMA, `total_cmp`).
- Investigated in the repository: no `[profile.*]`, no `.cargo/config.toml`; transcendental calls in `nest.rs` and `astro.rs`; `SNAPSHOT_PRECISION = 6` with `total_cmp` ordering (sound); CI verifies on two platforms without comparing outputs; a per-frame allocation in `PreviewCache::sync`.
- Added section 4.10 (three-tier determinism contract, numerics rules, profiles, measured build speed, hot-path probes, `profiling` feature, how budgets are measured). Added the profile and the cross-platform `--verify` diff to M2; the allocation fix to M3 (via section 4.10 text). Added open question 8.
- Refactor of the draft: section 4.10 names the `profiling` feature as the concrete instance of the section 4.8 "dev tooling" allowance, so the two sections point at each other instead of repeating the rule.

### Loop 3 (2026-10-02): testing techniques

- Searched: snapshot testing (`insta`, `expect-test`, golden-file review workflow and CI behavior), property-based testing (`proptest`: properties versus examples, shrinking, persisted regressions), `cargo-nextest` and `cargo-llvm-cov`, doc-test guidance.
- Investigated in the repository: 72 inline tests, none in `universe-app`; `--verify`/`--capture` checked only by the validator script; determinism asserted on three seeds; no property tests, golden files, or `App`-level tests.
- Added section 4.11 (golden tests with `UPDATE_GOLDEN`, one printer, `proptest` properties, Bevy-side tests, doc tests, hermetic tests, runner and coverage by need, failing-test-first). Rewrote M0 so the baseline is a committed golden test instead of untracked evidence; the "proof of unchanged" paragraph now points at that test. Added `proptest` to M2 and the golden-file immutability check to M7.
- Refactor of the draft: M0 doubles as the first tier-2 determinism probe, which removes the separate artifact-diff step added in loop 2 from M2 (one mechanism instead of two).

### Loop 4 (2026-10-02): API design and idioms

- Searched: Rust API Guidelines checklist (naming, common traits, newtypes, validation, `must_use`, documentation sections), Bevy 0.17 to 0.19 conventions (Messages versus Events, `Systems` suffix for system sets, `Single` parameter), Clippy cast lints and `unreachable_pub`.
- Investigated in the repository: `Level` as the existing validated newtype; bare `[f64; 3]` positions in two unit frames; 60 numeric `as` casts; 109 `pub fn` versus 3 `pub(crate)`; no `must_use`, `Display`, `From`, or `SystemSet`; Messages API already correct.
- Added section 4.12 (eleven rules, six of them CI-checked by lints). Extended the lint list in section 4.6; added types-only hardening to M3 and the `DiveSystems` set, camera `Name`, and `Single<&Window>` to M4.
- Refactor of the draft: M3's newtype step carries an explicit deferral clause so the behavior-preserving promise is never traded for a bigger diff; the rule for new code applies regardless.

### Loop 5 (2026-10-02): dependencies, supply chain, CI hygiene

- Searched: `cargo-deny`, `cargo-audit`, `cargo-vet` and when each is worth it; Dependabot and Renovate for Cargo lockfiles; `rust-version`; `cargo-machete`, `typos`; release profile options; GitHub Actions hardening (SHA pinning, token permissions, concurrency) and `Swatinem/rust-cache`.
- Investigated in the repository: 423 locked packages with no scanning; tag-pinned actions, no `permissions`, no `concurrency`, no build cache; no Dependabot; no `rust-version`, `license`, or `publish = false`; no `LICENSE`, `.editorconfig`, or `typos` config; strengths already present (exact pins, committed lockfile, `--locked` everywhere).
- Added section 4.13 (eight rules: version policies, advisory and license checks, bot updates with pins protected, manifest metadata, hardened and cached CI, conservative release profile, small hygiene tools with reasons, written-down commit conventions). Extended M2 with the manifest and CI items. Added open question 9 (license, an owner-only choice).
- Refactor of the draft: open question 8 now points at the M0 golden test (its real trigger since loop 3) instead of "the M2 diff"; question 9 placed after 8.

### Closing pass (2026-10-02)

- Section 1 states four questions instead of three (techniques and practices added) and names the CI-checked / convention tagging and the golden test as the two mechanisms that keep the draft's promises.
- Section 2 gained one summary bullet pointing at the per-section "State today" paragraphs; section 3 says where the technique sources live. The layout tree in 4.1 lists every new file the loops introduced. Section 4.5's list of `docs/engineering.md` sections, section 6 (non-goals), and section 7 (owner checks) were brought in line with sections 4.9 to 4.13.
- Verified: Markdown lint rules (no trailing whitespace, no tabs, one trailing newline, LF endings); section numbering; open questions 1 to 9 in order.

### Owner feedback pass (2026-10-02): instruction files as the project's memory

- Owner's point: the instruction files must be well organized and must carry everything this work produced, so that future sessions consider all of it.
- Rewrote section 4.5 from three bullets into the specification of the instruction layer: one job per file with a size discipline; stable `E-` rule IDs; a completeness requirement (traceability table, research kept as the References appendix and as ADR 0009 context); loading guaranteed by `AGENTS.md`, the `implementation`, `technical-validation`, and `planning` skills, and the pull request template; a re-review cadence tied to pin changes and once a year; CI checks on the section list, ID existence, and pointer-only files; the proposed always-on block for `AGENTS.md`.
- M1 now carries the ID assignment, the appendix, ADR 0009, the traceability table, the skill and template updates, and the extended drift guard. Section 7 gains the "fresh session" check: a new agent with no history must be able to recover every rule and its origin from the repository alone.
