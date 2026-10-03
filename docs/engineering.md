# Engineering rules

The complete technical conventions of this project. Every rule has a stable
`E-` identifier and is tagged either `CI-checked (<check>)`, naming the
machine that enforces it, or `convention`, meaning a reviewer enforces it.
An identifier is never reused; a retired rule keeps its identifier with a
note naming the ADR that retired it.

Source: Issue #85 (spec v1 approved, owner decisions Q1-Q9 applied).

## Toolchain and pins

Last reviewed 2026-10-02.

- `E-PIN-TOOLCHAIN`: Rust stable `1.97.1` exact, pinned in
  `rust-toolchain.toml`, edition 2024 everywhere. Convention (the pin file
  itself is the record; changing it is `E-PIN-ADR`).
- `E-PIN-BEVY`: Bevy exact `=0.19.1` with `default-features = false`,
  declared once in the root `Cargo.toml` under `[workspace.dependencies]`;
  every enabled feature has a written reason in `docs/universe/stack.md`.
  Convention.
- `E-PIN-ADR`: exact pins (Rust, Bevy) change only through an ADR.
  Convention (reviewer checks the ADR exists; bots are configured to ignore
  the pins, `E-BOT`).

## Workspace layout

Last reviewed 2026-10-02.

- `E-WORKSPACE-DEPS`: dependency versions are declared once in
  `[workspace.dependencies]`; crates inherit with `*.workspace = true`.
  Convention.
- `E-WORKSPACE-META`: `[workspace.package]` carries
  `rust-version = "1.97.1"`, `license = "MIT OR Apache-2.0"`, `repository`,
  and `publish = false`, inherited by every crate.
  CI-checked (cargo-deny).
- `E-LAYOUT`: the repository follows the layout in notion section 4.1:
  `crates/` for packages, `docs/` for documents, `scripts/` for repo
  tooling, no `examples/`, `benches/`, or `xtask` until a concrete need
  exists (`E-NO-XTASK`). Convention.

## Crate rules

Last reviewed 2026-10-02.

- `E-CORE-NO-BEVY`: `universe-core` is std only. It never depends on Bevy,
  wgpu, winit, or any rendering or windowing crate, and does no I/O beyond
  returning strings. CI-checked (dependency direction) from M2.
- `E-DEPS-DIRECTION`: dependency direction is `core <- render <- app`,
  never otherwise. CI-checked (dependency direction) from M2.
- `E-RENDER-SCOPE`: `universe-render` holds Bevy plugins, resources that
  wrap core types, systems, gizmo drawing, input handling, camera, and
  colors. Universe logic that could run without a window does not live
  here (known violation at M1: navigation and labels; resolved in M3).
  Convention.
- `E-RENDER-NO-MESH`: `universe-render` creates no meshes, materials, or
  textures (indicators only). Convention, reviewed in PRs.
- `E-APP-THIN`: `universe-app` holds `main`, CLI flag parsing, window
  setup, and the `--verify` / `--capture` drivers. It holds no universe
  logic and no drawing code. Convention.
- `E-CORE-API`: `universe-core` is the only crate whose public API is meant
  to be stable inside the workspace. `universe-render` exposes exactly its
  plugins plus the helpers `universe-app` needs. `universe-app` exposes
  nothing. Convention.
- `E-CORE-HEADLESS`: everything `--verify` needs is reachable from
  `universe-core` alone (true from M3 on). CI-checked (dependency
  direction on the verify driver) from M3.

## Module and file rules

Last reviewed 2026-10-02.

- `E-MOD-DOMAIN`: modules split by domain, not by kind. A module owns its
  types, systems, and tests together. No `components.rs` / `systems.rs`
  buckets. Convention.
- `E-PLUGIN-ONE-PER-MOD`: one plugin per module in `universe-render`. The
  module name is the plugin's domain (`camera`, `input`, `draw`); the plugin
  type is `<Domain>Plugin`. A parent plugin may group them and must declare
  the system order explicitly with `SystemSet`s or `.chain()`, never
  implicitly. Convention.
- `E-PLUGIN-STRUCT`: the struct-based plugin style (`UniverseRenderPlugin`,
  `DivePlugin`) stays; struct and function plugins are not mixed.
  Convention.
- `E-FILE-SIZE`: aim for 300 to 600 lines per file; split before 1,000.
  Tests count. CI-checked (doc drift guard: warns over 800 lines, fails
  over 1,200; thresholds in `.agent/project-config.json`) from M1.
- `E-CRATE-TOC`: each crate root (`lib.rs` / `main.rs`) begins with a
  crate-level doc comment that lists its modules in one line each.
  Convention.
- `E-TEST-PLACE`: unit tests live in the file they test
  (`#[cfg(test)] mod tests`). Integration tests that exercise a crate's
  public API live in that crate's `tests/`. The headless journey replay is
  the integration test of record and lives in core. Convention.

## Naming, visibility, and API design

Last reviewed 2026-10-02.

- `E-VIS-PUBCRATE`: `pub(crate)` is the default visibility. `pub` is
  reserved for items another crate uses. CI-checked (workspace lints:
  `unreachable_pub`) from M2.
- `E-NEWTYPE`: anything with units or an invariant is a newtype (pattern:
  `Level`). New code introduces `Seed(u64)`, `MarkerIndex(u32)`, and
  unit-tagged positions so the compiler, not a comment, keeps frames apart.
  Conversion between frames is a named method, never arithmetic at the call
  site. Convention (types-only migration in M3, proven by `E-GOLDEN`).
- `E-CAST`: numeric casts are deliberate. Lossless conversions use
  `From`; checked conversions use `TryFrom`; `as` appears only with
  `#[expect(clippy::cast_..., reason = "...")]` naming this rule. The
  `f64 -> f32` conversion at the render boundary (`to_vec3`) stays the only
  such site in `universe-render`. CI-checked (workspace lints: cast family)
  from M2.
- `E-TRAITS`: every public type derives `Debug` and `Clone`; `Copy` when
  small and plain; `PartialEq`, and `Eq` + `Hash` when it contains no
  floats; `Default` when a neutral value exists; `PartialOrd`/`Ord` only
  when the order means something (`Level`). `Display` only for user-facing
  text, never as a second `Debug`. Convention.
- `E-MUST-USE`: pure functions whose result is the whole point
  (`dive_step`, `should_open`, `preview_set`, `path_seed`) carry
  `#[must_use]`. CI-checked (workspace lints: `must_use_candidate`) from
  M2.
- `E-CTOR`: constructors validate (`new` returns `Option` or `Result`
  when an input can be invalid); conversions use `From`/`TryFrom` instead
  of ad-hoc `to_x` methods; method names follow `as_` (free, borrowed),
  `to_` (costly), `into_` (consuming). Convention.
- `E-STRUCT-FIELDS`: plain-data types keep public fields (`Generated`,
  `JourneyStep`); types with an invariant keep private fields and named
  methods. No getter/setter pairs that merely forward. Convention.
- `E-ENUM-BOOL`: a new `bool` parameter that is not obviously a predicate
  becomes a two-variant enum. Convention.
- `E-DOC-SECTIONS`: a function returning `Result` has an `# Errors`
  section; one that can panic has `# Panics` (after `E-NO-PANIC` there are
  none outside tests). CI-checked (workspace lints: `missing_errors_doc`,
  `missing_panics_doc`) from M2.
- `E-BEVY-NAME`: plugins end in `Plugin`; system sets end in `Systems`
  (`DiveSystems::{Input, Camera, Draw}`); components and resources carry no
  suffix; marker components are unit structs. Convention.
- `E-MSG-EVENT`: input and anything "many per frame, read later" is a
  `Message`; a state change other plugins react to immediately is an `Event`
  with an observer; a type never implements both. Convention.
- `E-ENTITY-NAME`: spawned entities carry a `Name` (lands in M4 for the
  camera) so inspectors and logs can say which entity. Convention.
- `E-SINGLE-WINDOW`: prefer the `Single<&Window>` system parameter, which
  skips the system when there is not exactly one window, over
  `Query<&Window>` plus a manual `single()` early return (lands in M4).
  Convention.
- `E-NO-UNSAFE`: no `unsafe`. CI-checked (workspace lints:
  `unsafe_code = "forbid"`) from M2. Lifting it is a
  decision (ADR); the only acceptable reason is a measured hot-path need
  that safe code cannot meet.

## Documentation

Last reviewed 2026-10-02.

- `E-DOC-INDEX`: `docs/README.md` is the index: one line per file or folder
  directly under `docs/`, kept current. CI-checked (doc drift guard) from
  M1.
- `E-ARCH-MAP`: `docs/ARCHITECTURE.md` has four sections (Bird's-eye view,
  Entry points, Codemap per crate with invariants and boundaries,
  Cross-cutting concerns). It names files, modules, and types but does not
  link them. Convention.
- `E-ARCH-INV`: architecture invariants are written in
  `docs/ARCHITECTURE.md` as **Architecture Invariant** callouts.
  Convention.
- `E-ADR-ONE`: one ADR per architecturally significant decision, in MADR
  form, at `docs/decisions/NNNN-title.md`. Status is one of `proposed`,
  `accepted`, `superseded by NNNN`. Superseded records stay, never deleted.
  Each ADR names the Issue it came from. CI-checked (doc drift guard:
  naming, unique consecutive numbers) from M1.
- `E-STACK-PINS`: `docs/universe/stack.md` keeps the pins and the Bevy
  feature table; its crate and architecture rule sections moved to
  `docs/ARCHITECTURE.md` and here (Q6), leaving a pointer. Convention.
- `E-LADDER-FROZEN`: `docs/universe/ladder.md` changes only with owner
  approval. Convention.
- `E-POINTER-ONLY`: `CLAUDE.md` and `CONTRIBUTING.md` are pointers only:
  every non-heading line contains a path or link, at most eight lines.
  CI-checked (doc drift guard) from M1.
- `E-AGENTS-REF`: `AGENTS.md` names `docs/engineering.md` and
  `docs/ARCHITECTURE.md` under References and carries the always-on block.
  CI-checked (doc drift guard) from M1.
- `E-REVIEW-CADENCE`: sections are re-reviewed against current sources when
  the Bevy or Rust pin changes, otherwise once a year, as a `chore/` pull
  request listing what was re-checked and what changed. Convention.

## Testing ladder and techniques

Last reviewed 2026-10-02.

- `E-TEST-LADDER`: cheapest rung first: pure function or type test, no ECS
  (the default); raw `World`; `World::run_system_once`; `Schedule::run`
  when order matters; `App` with `MinimalPlugins` plus the plugin under
  test, stepped with `app.update()`, with an iteration cap. Convention.
- `E-TEST-NO-WINDOW`: no test opens a window or needs a GPU. Convention.
- `E-REPLAY-CORE`: the headless journey replay (`replay_autopilot`) is the
  integration test of record and lives in core. Convention.
- `E-GOLDEN`: the headless protocol is pinned by golden files under
  `crates/universe-app/tests/golden/`. `UPDATE_GOLDEN=1` rewrites them;
  CI never sets it, so an unreviewed change fails. Golden files are
  rewritten only when the pull request explains the change. CI-checked
  (golden test) from M0.
- `E-ONE-PRINTER`: every snapshot and every golden comparison goes through
  `snapshot_generated` and the `protocol` module; no test formats floats on
  its own. (The `protocol` module lands in M5; the snapshot half holds at
  M1.) Convention.
- `E-PROTEST`: `proptest` is a dev-dependency of `universe-core`
  (dev-dependencies do not ship and do not count against "std only") with
  the five named properties from notion 4.11; `proptest-regressions/` is
  committed the first time proptest writes one. CI-checked (cargo test).
- `E-BEVY-TEST`: an `App`-level test and a schedule-order test live in
  `universe-render` (lands in M4); the plugin set is documented next to the
  test. CI-checked (cargo test) from M4.
- `E-DOC-TEST`: a public function whose contract is not obvious from its
  signature carries one doctest example; everything else is a unit test.
  CI-checked (doc tests).
- `E-TEST-HERMETIC`: tests are hermetic. No wall clock, no randomness
  without a fixed seed, no files outside a per-test temporary directory
  (`std::env::temp_dir()` plus the test name), no network. A test that
  needs the GPU does not exist. Convention.
- `E-BUG-TEST-FIRST`: every bug fix adds the failing test first, at the
  lowest rung of `E-TEST-LADDER` that reproduces it, named after the
  symptom; correction rounds cite that test on the Issue. Convention.

## Cargo features policy

Last reviewed 2026-10-02.

- `E-FEATURE-CONSUME`: every dependency is declared
  `default-features = false` with an explicit feature list, and each
  enabled feature has a written reason. Convention.
- `E-FEATURE-DEFINE`: a crate defines a feature only for an optional
  dependency, a platform or backend choice, or developer-only tooling. A
  feature is never used for a runtime choice (`--verify` and `--capture`
  stay CLI flags), for architectural separation (that is a crate boundary),
  or for product variants. Convention.
- `E-FEATURE-ADDITIVE`: enabling a feature never removes or changes
  behavior that exists without it. Every combination must compile and pass
  tests. CI-checked (feature matrix, dormant until the first feature
  exists).
- `E-FEATURE-DECLARED`: a feature is declared in three places: a one-line
  comment in `Cargo.toml`, a line in the crate-level doc comment, and a row
  here. The `default` feature is always written out, even when empty.
  Convention.
- `E-FEATURE-NONE`: today no crate defines a feature and none is needed.
  The Bevy Book's `headless` feature is a workaround for single-crate
  projects; this project is headless by construction because
  `universe-core` has no Bevy dependency. Convention.

## Error handling and diagnostics

Last reviewed 2026-10-02.

- `E-NO-PANIC`: no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`,
  or possibly-out-of-bounds indexing outside tests. CI-checked (workspace
  lints: panic family at deny, `allow-unwrap-in-tests`,
  `allow-expect-in-tests`, `allow-panic-in-tests`,
  `allow-indexing-slicing-in-tests` in `clippy.toml`) from M2.
  Every silenced lint carries a reason naming an `E-` ID.
- `E-OPT-RESULT`: `Option` when absence is a normal outcome with one
  obvious meaning; `Result<T, E>` with a named error type when the caller
  needs the reason or must report it. Silent early return on `None` only
  when absence is normal. Convention.
- `E-ERR-TYPE`: `universe-core` error types are small `enum`s with
  hand-written `Display` and `std::error::Error` impls, one per module that
  needs one, named `<Module>Error`. `universe-app` is the only crate allowed
  a type-erased error. Convention (`thiserror`/`anyhow` stay in "adopt
  when needed").
- `E-SYS-RESULT`: a system that can fail for a reason that is a bug returns
  `Result` and uses `?`; the app installs `panic` in debug builds and
  `error` in release builds (lands in M5). Systems that early-return on
  normal absence keep `let ... else { return }`. Convention.
- `E-LOG-TRACING`: human-facing diagnostics use the `bevy::log` macros
  (`info!`, `warn!`, `error!`, `*_once!` inside per-frame systems). Default
  filter `warn,universe_core=info,universe_render=info,universe_app=info`;
  `RUST_LOG` overrides it. Structured fields instead of formatted strings.
  Convention.
- `E-VOCAB`: the words error, panic, warn, deprecated, FAILED are reserved
  for real problems anywhere in program output. Informational lines never
  contain them. CI-checked (output vocabulary; also scanned by the
  validator run).
- `E-BACKTRACE`: CI and the validator run with `RUST_BACKTRACE=1`.
  CI-checked (workflow file).

## Headless protocol

Last reviewed 2026-10-02.

- `E-STDOUT-PROTOCOL`: stdout is the machine protocol:
  `VERIFY-OK` / `VERIFY-FAIL`, `CAPTURE-OK` / `CAPTURE-FAIL`, and the
  `LEVEL`, `SNAPSHOT`, `JOURNEY`, `RATIO`, `PREVIEW` lines. Exact formats
  are specified below; changing one is a decision (ADR) because the
  validator and the golden files depend on them. CI-checked (golden test)
  from M0.
- `E-EXIT-CODE`: exit 0 when every check passed, 1 when a check failed, 2
  when the command line itself was wrong (lands in M5; Q7). CI-checked
  (headless gate probes) from M5; 0/1 at M1.
- Line formats (`--verify`): `LEVEL <label> [<milestone>] scale=<scale>
  anchor="<anchor>" path=<chain> seed=<seed> markers=<n>
  determinism=<PASS|FAIL> in-budget=<PASS|FAIL> marker-exists=<PASS|FAIL>
  replay-match=<PASS|FAIL> <PASS|FAIL>`; `SNAPSHOT <label> <snapshot>`;
  `JOURNEY opened=<label> marker=<n> at=<s>s`;
  `JOURNEY reached=<label> levels-opened=<n> <PASS|FAIL>`;
  `RATIO L<n>->L<n+1> child/parent=<ratio> markers<=<max> <PASS|FAIL>`;
  `PREVIEW entering=<label> previewed=<n> (cap <n>)
  position-error=<e> alive=<n> (max <n>) <PASS|FAIL>`;
  `PREVIEW brightness-continuous:<PASS|FAIL> endpoints:<PASS|FAIL>
  all-openings:<PASS|FAIL> <PASS|FAIL>`;
  `INVERSE closed=<n> at-root:<PASS|FAIL> one-level-round-trip:<PASS|FAIL>
  landed-inside-start:<PASS|FAIL> <PASS|FAIL>`; final `VERIFY-OK` or
  `VERIFY-FAIL`. (`--capture`): `CAPTURE-OK files=<n> dir=<dir>` or
  `CAPTURE-FAIL ...`; per-level `level-<L>.txt` and `frame-<L>.ppm` files
  plus `capture.log`.

## Numerics and determinism

Last reviewed 2026-10-02.

- `E-DET-TIERS`: determinism has three tiers: same binary on the same
  machine is byte-identical (CI-checked by the repeat run); same source on
  Linux and Windows gives byte-identical snapshots at six decimals
  (CI-checked by the golden test on both jobs from M0); a different
  compiler version is not promised. Convention.
- `E-TRANSCENDENTAL`: transcendental calls (`sin`, `cos`, `exp`, `ln`,
  `powf`, `asin`) are allowed only in `universe-core` generators. If the M0
  golden test shows a platform difference, the pure-Rust `libm` crate is
  added to `universe-core` as a documented exception to std-only (Q8).
  Convention.
- `E-NO-TARGET-CPU`: never `-C target-cpu=native` or any flag that changes
  floating-point codegen between machines; `mul_add` only when the fused
  result is wanted and documented. Convention.
- `E-FLOAT-SORT`: floats never cross a sort or a hash un-canonicalized.
  `total_cmp` for ordering, explicit rounding to six decimals before any
  text comparison, no `f32` in `universe-core` positions (conversion to
  `f32` happens at the render boundary only). Convention.

## Performance and profiling

Last reviewed 2026-10-02.

- `E-PROFILE`: `[profile.dev] opt-level = 1` with `package."*"` at 3;
  `[profile.release] lto = "thin", codegen-units = 1, strip = "debuginfo"`,
  `panic = "unwind"`; `[profile.profiling]` inherits release with
  `debug = 1`. Committed, so every machine and CI build the same way.
  CI-checked (golden test proves the profiles move no float
  result) from M2.
- `E-BUILD-TIMED`: build speed is measured, not guessed.
  `cargo build --timings` before and after any crate split (M3-M5); the two
  numbers are posted on the sub-issue. Convention.
- `E-HOT-NOALLOC`: no allocation per frame in `Update` systems (probes:
  `PreviewCache::regenerations`-style counters); gizmo drawing stays
  immediate-mode and bounded by the live-generation cap
  (`2 + PREVIEW_CAP`); any per-frame `Vec` becomes a reused buffer or a
  fixed-size array. The `PreviewCache::sync` allocation is fixed in M3 with
  a test asserting zero regenerations and zero re-allocations for a static
  camera. Convention.
- `E-BUDGET`: budgets stay in `docs/universe/stack.md` (60 fps target;
  draw-call and texture budgets TBD). This document only says how each is
  measured: frame time from Bevy's `FrameTimeDiagnosticsPlugin` behind the
  profiling feature; generation time as a `TIMING` line (adopt when needed),
  informational, never pass/fail. Convention.

## Lint and format policy

Last reviewed 2026-10-02.

- `E-FMT`: `rustfmt.toml` (edition 2024, default style) is committed so
  local and CI agree. CI-checked (cargo fmt --check) from M2.
- `E-CLIPPY`: `clippy.toml` (`allow-unwrap-in-tests`,
  `allow-expect-in-tests`, thresholds) is committed. CI-checked
  (cargo clippy with `-D warnings`) from M2.
- `E-LINTS`: `[workspace.lints.rust]` (`missing_docs`, `unreachable_pub`,
  `unused_qualifications` at warn, `unsafe_code = "forbid"`) and
  `[workspace.lints.clippy]` (panic family at deny; cast family,
  `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc` at warn),
  with `[lints] workspace = true` in each crate. CI-checked
  (clippy) from M2.
- `E-DOC-BUILD`: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --no-deps --locked` catches broken intra-doc links and missing docs.
  CI-checked (cargo doc) from M2.

## Dependencies and supply chain

Last reviewed 2026-10-02.

- `E-VERSION-POLICY`: three kinds of version, three policies. Toolchain and
  Bevy stay exact pins, changed only by an ADR. Other direct dependencies
  use caret requirements in `[workspace.dependencies]`, pinned by
  `Cargo.lock`. Transitive dependencies are owned by the lockfile.
  `cargo update` is never casual: a lockfile change is its own pull request
  with the `cargo tree` diff in the body. Convention.
- `E-DENY`: `deny.toml` covers advisories (RustSec, yanked, unmaintained),
  licenses (allow-list starting from MIT, Apache-2.0, BSD-2-Clause,
  BSD-3-Clause, ISC, Zlib, Unicode-3.0, extended only by ADR), bans (bevy
  wrapped by `universe-render` and `universe-app`; `multiple-versions =
  "warn"`), and sources (crates.io only). CI-checked
  (cargo deny) from M2.
- `E-BOT`: `.github/dependabot.yml` proposes `github-actions` (weekly) and
  `cargo` (weekly, lockfile-only, one group) updates, ignoring Bevy and
  anything `=`-pinned. Bot pull requests are `dependabot/`
  branches treated as "no parent Issue". Convention.

## CI

Last reviewed 2026-10-02.

- `E-CI-HARDEN`: every third-party action is pinned to a full commit SHA
  with the version in a trailing comment; the workflow declares
  `permissions: contents: read` at the top and widens per job only where a
  step needs more; a `concurrency` group keyed on workflow and ref cancels
  superseded runs; `Swatinem/rust-cache` runs after the toolchain step on
  both jobs; `RUST_BACKTRACE=1` is a job env. CI-checked
  (workflow file, validator) from M2.
- `E-CI-AUDIT`: a weekly scheduled workflow runs
  `cargo deny check advisories` and nothing else. Convention.
- `E-COMMIT`: branch names match `^(feat|fix)/[0-9]+-[a-z0-9-]+$` (plus
  `dependabot/**` from M2); commit subjects are imperative, under 72
  characters, naming the Issue when one exists. CI-checked (branch name
  guard) from M1.
- `E-TYPO`: `typos.toml` at the root; one CI step runs the spell check.
  CI-checked (typos) from M2.
- `E-EDITORCONFIG`: `.editorconfig` (UTF-8, LF, final newline, no trailing
  whitespace) so editors prevent what the Markdown lint rejects. Convention.

## Adopt when needed

Last reviewed 2026-10-02. Each row names the trigger that adopts it; until
then the tool is deliberately absent.

| Tool | Trigger |
|---|---|
| `TIMING` line in `--verify` output | First generation-time measurement need (informational, never pass/fail) |
| `profiling` Cargo feature (`bevy/trace_tracy`) and `[profile.profiling]` use | First profiling need (the one legitimate feature per `E-FEATURE-DEFINE`) |
| `FrameTimeDiagnosticsPlugin` | First frame-time measurement need |
| `cargo llvm-cov` in the job summary (informational, never a gate) | A sub-issue questions test reach |
| `cargo hack` feature matrix (`E-FEATURE-ADDITIVE`) | First Cargo feature is defined |
| `cargo nextest` (doctests stay on `cargo test --doc`) | Suite over a minute, or flakiness detection needed |
| `tracing` / `bevy::log` macros in systems | First human-facing diagnostic (none exists today) |
| Unit-tagged position newtypes beyond M3 | M3 deferral clause triggers (ninth sub-issue) |
| `thiserror` | Third hand-written error impl appears |
| `anyhow` (allowed in `universe-app` only) | Owner approves the dependency |
| `cargo-vet` | External consumers require audited trust |
| `cargo-machete` | A crate grows more than a handful of dependencies |
| mdBook | Documentation outgrows plain Markdown |
| `xtask` | A task needs the Rust toolchain itself |
| `examples/`, `benches/` | A concrete need exists |
| `CHANGELOG.md` | Issues stop being the record (not planned) |

## How to add a crate, a plugin, a feature, a dependency, a decision

Last reviewed 2026-10-02.

- A crate: it must answer one of the four questions in the Goal section
  with a new dependency direction; otherwise it is a module. Record in an
  ADR, extend the `E-DEPS-DIRECTION` check, add the codemap entry.
  Convention.
- A plugin: one module, one `<Domain>Plugin`, system order explicit
  (`E-PLUGIN-ONE-PER-MOD`); document the plugin set next to the
  `E-BEVY-TEST` test. Convention.
- A feature: allowed only per `E-FEATURE-DEFINE`; declare in three places
  (`E-FEATURE-DECLARED`); adding the first feature also wires the dormant
  matrix (`E-FEATURE-ADDITIVE`). Convention.
- A dependency: caret requirement in `[workspace.dependencies]` with a
  written reason; lockfile change as its own pull request
  (`E-VERSION-POLICY`); license on the allow-list or an ADR extending it
  (`E-DENY`). Convention.
- A decision: one MADR file per `E-ADR-ONE`; the pull request answers the
  template's rule question. Convention.

## References

Last reviewed 2026-10-02. Section 3 of the notion plus the per-section
sources, grouped by topic.

- Architecture and organization: Bevy Book Project Organization
  (https://bevy.org/learn/book/modular-architecture/project-organization/),
  Bevy Book Testing Bevy Apps
  (https://bevy.org/learn/book/development-practices/testing/), Bevy Book
  Building Bevy's Ecosystem
  (https://bevy.org/learn/quick-start/plugin-development/), Bevy best
  practices (https://github.com/tbillington/bevy_best_practices), matklad
  ARCHITECTURE.md (https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html),
  MADR (https://adr.github.io/madr/), Cargo workspaces
  (https://doc.rust-lang.org/cargo/reference/workspaces.html).
- Lints: Clippy lint groups (https://doc.rust-lang.org/clippy/lints.html).
- Supply chain: cargo-deny (https://github.com/EmbarkStudios/cargo-deny),
  large Rust repositories (Zed, Veloren, rust-analyzer layouts).
- Error handling: Rust API Guidelines, anyhow and thiserror documentation
  (library/application split), Clippy `unwrap_used` / `expect_used`
  documentation, Bevy `bevy::ecs::error` documentation, Bevy `LogPlugin`
  documentation.
- Performance: Bevy Book "Fast Compiles" and "Compiling Release Builds",
  Bevy `docs/profiling.md`, Rust `f64` documentation (unspecified precision
  of transcendental functions), Cargo profiles reference.
- Testing: insta and expect-test documentation, proptest book,
  cargo-nextest and cargo-llvm-cov documentation, rust-analyzer
  architecture.md (data-driven tests).
- API design: Rust API Guidelines checklist, Bevy Book (visibility, plugin
  organization), Bevy 0.17-0.19 release notes and migration guides (Messages
  versus Events, `Systems` suffix), Clippy cast lints.
- Dependencies and CI: Cargo reference (`rust-version`, profiles),
  cargo-deny and cargo-audit documentation, RustSec guidance, GitHub Actions
  security hardening guidance, Swatinem/rust-cache documentation, Microsoft
  Rust engineering practices (supply chain).
