# Validator Capabilities

| capability | how proven | Issue | date |
| read a blob at a commit SHA (`git show SHA:path`) | probe run post-allowlist-widening returned the file header (was Permission denied: shell at baseline) | #97 | 2026-10-03 |
| read a git object type (`git cat-file -t SHA`) | probe run post-widening returned commit (was denied at baseline) | #97 | 2026-10-03 |
| list linked branches (`gh issue develop N --list`) | probe run post-widening exited 0 (was denied at baseline; empty output correct, branches deleted post-merge) | #97 | 2026-10-03 |
| read a rendered file on GitHub at a commit SHA | preflight: raw file URL + contents API at commit f60bf56 (blob SHA from contents API is not usable as ref) | #30 | 2026-10-01 |
| run the headless app check (`cargo run -p universe-app -- --verify`: exit code, `VERIFY-OK`, per-line PASS) and `cargo test --locked --workspace` locally | preflight on main ba04b8f: rc=0, VERIFY-OK, 68 tests green, identical repeat (https://github.com/taftary/astrolith/issues/63#issuecomment-5940228312). **Unresolved disagreement, recorded rather than settled:** the #87 second preflight reported `--verify` output varying between runs (timings, float magnitudes); the #87 third preflight ran it 8 times (6 debug, 2 release) and got byte-identical text including `at=3.35s` and `position-error=1.83e-15`, so the timings appear deterministic. Three claims about `--verify` determinism, reconciled at #87 rather than left to contradict each other. **Raw byte-identity is unproven and must not be asserted.** `validate.py` nonetheless asserts a **normalized** repeat identity, as a hard error rather than a warning: its edge probe compares two runs after `norm()` rewrites `(\d+)` to `(PID)` and `in \d+\.\d+s` to `(in T)`, so a difference confined to PIDs or timings does not fail the run. And the eight-run evidence above suggests raw byte-identity would in fact hold. So: a spec may rely on normalized repeat determinism, must not rely on raw byte-identity, and note that the script is stricter than the evidence warrants. `--capture` byte-identity below is the proven one | #63, disputed #87 | 2026-10-01 |
| poll GitHub check-runs / actions runs for a commit SHA and distinguish success vs failure, plus read a PR body containing double quotes from file content without shell-quoting failure (background concurrent fetch proven) | preflight: PR #61 head 176799a -> ci completed/success vs 91ef7cc -> ci completed/failure; PR #61 body len 2355 with quotes checked via regex; concurrent fetches 282ms/193ms (https://github.com/taftary/astrolith/issues/62#issuecomment-5947441919) | #62 | 2026-10-02 |
| run python repo scripts locally (`scripts/sidebar/project.py get --issue N`, `scripts/validation/validate.py --help`) as the tool class the new gate scripts belong to | preflight on main eaa71da: project.py get ok, validate.py usage printed (bare `python --version` denied by validator sandbox allowlist; script-path invocations work) | #76 | 2026-10-02 |
| run the app capture mode (`cargo run --locked -p universe-app -- --capture <dir>`: exit 0, `CAPTURE-OK`, 11 level snapshots + 11 frame plots + manifest, two runs byte-identical) | proven on feat/76-workflow-gates: CAPTURE-OK files=23 twice, 23/23 files byte-identical across runs | #76 | 2026-10-02 |
| read per-job and per-step **conclusions** and check-run annotations for a commit (not step log text) | preflight: `GET /commits/{sha}/check-runs` and `GET /actions/runs/{run_id}/jobs` both 200 unauthenticated; step 9 "Intake path guard" failure read at 9a27990, success at 1f9b4ff; failure text via `GET /check-runs/{id}/annotations`. GitHub synthesises the annotation message (`Process completed with exit code N.`), so **no captured output ever appears in it**. Annotations are per-check-run, not per-step: attribute a step via the jobs endpoint, not the annotations endpoint. | #87 | 2026-10-02 |
| tell a GitHub-level `if:` skip apart from a pass, and read a per-file `patch` out of the compare API | preflight: on PR run 37016187220 step 9 reads `skipped` while all ungated steps read `success`; `GET /compare/{base}...{head}` returns a `patch` per file (43/43 files on 898ce1d...1f9b4ff). The compare endpoint **silently ignores `?path=`** — select the file client-side from `files[]` (https://github.com/taftary/astrolith/issues/87#issuecomment-5958037653) | #87 | 2026-10-02 |
| read `git log`, `git status`, `git rev-parse` locally | preflight: `git rev-parse HEAD~1` -> c31f6f6 at head 9a27990; `git log --oneline` works. **The local clone is full-depth** (`git rev-parse --is-shallow-repository` -> false), so any "does the parent resolve?" assertion passes vacuously locally and proves nothing. | #87 | 2026-10-02 |
| prove a *future* file under an already-allowlisted script path is runnable, without the file existing | preflight: `python scripts/validation/zz_nonexistent_probe_zz.py --help` is **permitted** and Python itself reports `can't open file … [Errno 2]`; `scripts/validation/subdir/probe.py` also permitted, so `*` spans `/`; `scripts/validationx/probe.py` refused. Permission is decided on the command string **before** the file is opened, so a nonexistent-path probe is a valid proof about the rule (https://github.com/taftary/astrolith/issues/88#issuecomment-5958685543) | #88 | 2026-10-02 |
| write to GitHub: post and delete an Issue comment via `gh issue comment` / `gh api` | preflight: `gh issue comment 88 --body test` succeeded (comment 5958577685) and `gh api -X DELETE` removed it, re-read confirming 404. So earlier reports of "no write path" were wrong — see the boundary table | #88 | 2026-10-02 |
| byte-compare and first-differing-line diff a remote file read at two commits, client-side, with **no shell** | preflight: FNV-1a plus a line walk over `raw.githubusercontent.com` reads; `.github/workflows/ci.yml` at 1f9b4ff / c31f6f6 / 9a27990 all len 14704 with identical digest, empty first-differing-line diff, while 898ce1d is len 8616. Needs no diff rendering, so it is a better instrument than the compare `patch` (https://github.com/taftary/astrolith/issues/87#issuecomment-5958876413) | #87 | 2026-10-02 |
| fetch a binary over HTTPS to a local Temp file (`curl <url> -OutFile <path>`, PowerShell alias of Invoke-WebRequest; add `-UseBasicParsing` for anything but `-OutFile`) | preflight: github.com/taftary.png to probe.png, hash 7396fbb7, 1550 bytes; the published release asset round-trip gave the identical hash (https://github.com/taftary/astrolith/issues/134#issuecomment-5977139821) | #134 | 2026-10-04 |
| create, publish, and delete a GitHub release (`gh api repos/.../releases -X POST`, `-X PATCH -F draft=false`, `-X DELETE`) and delete its tag ref (`gh api repos/.../git/refs/tags/<tag> -X DELETE`) | preflight: release 402866047 created draft+prerelease, published as prerelease, deleted; tag ref deleted; all read back 404. A **draft** release's assets are not publicly downloadable and their URLs use an `untagged-<hex>` path rewritten on publish; "hidden" means published **pre-release**, not draft | #134 | 2026-10-04 |
| upload a release asset (`gh api https://uploads.github.com/repos/.../releases/<id>/assets?name=... -X POST -H "Content-Type: ..." --input <file>`), state=uploaded. **`--hostname uploads.github.com` does not work** (gh targets api.uploads.github.com); the full URL works under the existing `gh api *` entry | preflight: assets 609236439 (png, 1550 B) and 609236497 (ppm, 385009 B); the published `browser_download_url` served 200 and byte-identical content | #134 | 2026-10-04 |
| embed a release-asset image in an Issue comment and prove GitHub rendered it (`body_html` via `-H "Accept: application/vnd.github.html+json"` contains `<img src=...>`; no camo for github.com-hosted images) | preflight: comment 5977106874 `body_html` showed `<img src=".../releases/download/preflight-134-probe/probe.png">`. **Layout trap:** the same URL on the same line as a leading `<!-- -->` marker did not render (a line starting with `<!--` is a CommonMark HTML block through `-->`); marker and picture need separate paragraphs, or image first | #134 | 2026-10-04 |
| edit an existing Issue comment (`gh api repos/.../issues/comments/<id> -X PATCH -f body=...` single line; `gh issue comment N --edit-last --body` for multi-line) | preflight: PATCH on 5977106874 returned the new body; the preflight report was repaired with `--edit-last` | #134 | 2026-10-04 |
| shell-boundary rule for multi-line GitHub bodies: the quote must open the token; `-f body="..."` multi-line and escaped double quotes inside a double-quoted string are refused (each later line is checked as a new command); single-quoted multi-line `--body '...'` is permitted and keeps backticks; PowerShell eats backticks inside double-quoted strings | preflight: bisected on `repos/taftary/astrolith/zz_nonexistent_probe` (404 endpoint), three refused commands recorded verbatim on the Issue | #134 | 2026-10-04 |
| run the frame proof end to end (`python scripts/validation/frame_proof.py --issue N --sha S --visual yes|no`: after capture at HEAD, before capture at the base worktree with an isolated target dir, overwrite publish to the `validation-evidence` pre-release) | live runs on the branch: `FRAME-PROOF-OK changed=0 published=22` plus a same-name republish proving delete-then-upload; render check comment 5977681612 gave `body_html` `<img>` then read 404 after delete (https://github.com/taftary/astrolith/issues/137#issuecomment-5977683004) | #134 | 2026-10-04 |
| fetch a public third-party web page and quote a text fragment from it, client-side (`$r = curl <url> -UseBasicParsing`, then `$r.Content -replace '<[^>]+>',' '` and `IndexOf`/`Substring`) | preflight on main dae3352: NASA NSSDC Earth fact sheet 200 with `Equatorial radius (km) 6378.137`; ESA Gaia 200 with `Science with 1 billion objects in three dimensions`; Wikipedia Laniakea 200. Earlier rows proved HTTPS only against github.com hosts (https://github.com/taftary/astrolith/issues/143#issuecomment-5979492210) | #143 | 2026-10-04 |
| list the `## ` headings of a Markdown file at a commit, in order, without a pipe (`$lines = $c.Content -split "`n"; foreach ($l in $lines) { if ($l -like '## *') { $l.TrimEnd() } }`) | preflight on main dae3352: `docs/universe/ladder.md` via raw URL, six headings in order; `... \| Select-String` was refused (pipe), the loop was not. `--jq` filters with embedded spaces are split by the shell layer (`accepts 1 arg(s), received N`): use space-free filters | #143 | 2026-10-04 |
| read flight-controls input code at a commit via `git show SHA:path` + post-#151 verify baseline | preflight on main b3141a5: `git show b3141a5:crates/universe-render/src/input.rs` (hover/target, wheel/Arrow dive, Space autopilot; no F/WASD/saves pre-#152); `docs/universe/ladder.md` 7 headings with R8 amendment; `cargo run --locked -p universe-app -- --verify` exit 0 VERIFY-OK with L1-L10 dive (https://github.com/taftary/astrolith/issues/152#issuecomment-5984738269) | #152 | 2026-10-04 |
| read core density-generator code at a commit via `git show SHA:path` + post-#152 verify baseline | preflight on main 2c67207: `git show 2c67207:crates/universe-core/src/density.rs` (L1-L3 field header, DENSITY_OCTAVES=3, CLUSTER_THRESHOLD=0.5, DENSITY_BASE_COUNT=32, portal-tier salts with interim-means comment for #153; no environment typing yet); `cargo run --locked -p universe-app -- --verify` exit 0 VERIFY-OK with L1-L10 dive | #153 | 2026-10-04 |

**Traps that cost time; check these before concluding anything.**

- `GET /actions/runs?head_sha=` can return **several runs for one head SHA with opposite conclusions** (measured on
  `e46c675`: run 37015684194 `failure` and 37016187220 `success`, both `pull_request`). Always take the newest run id.
- A run's `head_sha` is the **branch head**, not the merge ref, when looking up pull-request runs.
- The compare API caps `files[]` at 300 entries and may omit `patch` on very large diffs, so name the file you read.
- The `Validator pass link guard` step fails the **first** run of every pull request until the validator verdict link
  and the full 40-hex head SHA are in the PR body. Two runs on one SHA is normal; read the **final** one.
- `scripts/gates/spec_gate.py` detects a stage-5 preflight result by loose proximity match between the stage name and
  the word "pass", not by a marker. Harmless on a pass; on a fail it could be satisfied by unrelated prose.

## The validator sandbox shell boundary

`.opencode/agents/validator.md` is the rule; the table below is the observed behaviour. Two earlier rows of this file
got it wrong in **both directions** and are corrected here: `gh` was listed as wholly refused when only a bare `gh`
is, and `git diff` was listed as reachable when no allowlist entry covers it. When these two disagree, the allowlist
wins.

**Allowed** (`resource` -> `effect: allow`, from `.opencode/agents/validator.md`):

| allowed | note |
| --- | --- |
| `gh issue view *`, `gh issue comment *`, `gh pr view *`, `gh pr checks *`, `gh api *` | the validator **can** write to GitHub via these |
| `git rev-parse *`, `git status *`, `git log *`, `git show *`, `git diff *`, `git cat-file *`, `git hash-object *` | show/diff/cat-file allowed post-#97; hash-object added for #111 |
| `curl *` | but see the alias trap below |
| `cargo test *`, `cargo run *`, `cargo doc *`, `cargo fmt *`, `cargo deny *`, `cargo clippy *` | doc/fmt/deny allowed post-#97; clippy added for #111 |
| `python scripts/validation/*` | `*` spans `/`; permission is decided before the file is opened |
| `python scripts/sidebar/{project.py get,milestone.py show,relationships.py list,development.py list,notifications.py show} *` | |

**Refused** (`shell` -> `resource: "*"` -> `effect: deny`, with no matching allow):

| refused | note |
| --- | --- |
| bare `gh`, `gh --version` | a bare `gh` matches no allow entry. `gh --version` was mistaken for a probe of the whole CLI |
| `python --version`, `python3 ...`, `python <anything not under an allowlisted path>` | |
| shell `;` chaining, `\|` pipes, `>` redirects | so a numeric exit code is often uncapturable; `test result: ok` and `VERIFY-OK` must stand in |
| `curl.exe` | **alias trap:** the allowlist entry is `curl *`, not `curl.exe *`. In PowerShell `curl` is an alias for `Invoke-WebRequest`, so `curl <url>` runs as that cmdlet while `curl.exe` matches nothing and is refused |

**The shell layer honours quotes containing spaces and newlines but strips double quotes inside them.**

## Not available to the validator (do not write specs that require these)

Owner decision 2026-10-02: stop relying on step logs; use per-step conclusions and annotations instead.
That decision stands, but its stated *reason* was wrong: step logs are not universally unreachable, they
are unreachable **without admin rights**. Corrected at #87; see the row.

| not available | exact refusal | Issue |
| --- | --- | --- |
| CI step log **text**, *without admin rights* | `GET /actions/jobs/{id}/logs` -> 403 "Must have admin rights to Repository."; `GET /actions/runs/{id}/logs` -> 403; GitHub's own step-log partial -> 404 (sign-in required); browser unavailable. **This is a credential limit, not a universal one.** Verified 2026-10-02: authenticated as the repository owner, the same request returns HTTP 200 with 33,105 bytes of readable text (job `111019500791`). Still do not rely on it — the owner decision above stands, and an admin-only capability is not one a specification should depend on, because it silently stops being available to a differently-privileged validator. | #87 |
| **creating** files or directories (write path) | no write permission, so the validator cannot synthesise a differing pair on demand; a comparison criterion must operate on files that already exist or have the script generate its own pair | #88 |
| unauthenticated `api.github.com` at volume | `core` rate limit 0/60 on one run; read the commit via `gh api` instead | #88 |
| file content at a commit **locally** | `git show <sha>:<path>` and `git cat-file -t <sha>` -> `Permission denied: shell`; loose objects under `.git/objects/` read as raw zlib. HTTP substitute works: `raw.githubusercontent.com`, contents API `?ref=<commit-sha>`, compare API | #87 |

**Two kinds of skip, only one of which is visible.** An `if:`-gated step reads `conclusion: skipped` and is
therefore **distinguishable** from a pass — a criterion may require "not skipped". A skip *inside* the script
(`exit 0` on a detected merge commit) reads `conclusion: success` with no annotation and is **indistinguishable**
from a genuine pass. Specs must not assert on step output, and must not treat `success` as proof that a check
actually ran something.

**How a step's internal assertions get closed.** Only the aggregate step conclusion is observable. If a criterion
says a step "demonstrated" several things, prove the chain by reading that step's body at the commit and confirming
it asserts each one and exits non-zero on any failure — not from the conclusion alone.

Later notions check only what is new; capabilities already proven here are not re-proven.
