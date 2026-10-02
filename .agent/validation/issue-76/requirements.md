# Requirements — issue #76 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/76#issuecomment-5951150204,
approval: https://github.com/taftary/astrolith/issues/76#issuecomment-5951223345).
Owner approval of the spec confirms the criteria text below.
Process-layer criteria use file probes (worktree reads); runtime-layer
criteria use log probes. Probes are the AI's addition; the owner never
sees this syntax.

- [ ] C1: The stage-to-skill table is written down once, with wait-checks as a helper (verify: file docs/workflow.md contains helper skill with no stage)
- [ ] C2: The stale plan carries a historical header and is not a live reference (verify: file docs/plans/workflow.md contains HISTORICAL)
- [ ] C3: The three gate scripts exist and skills call them instead of prose (verify: file scripts/gates/merge_gate.py contains MERGE OK)
- [ ] C4: CI fails a pull request with a missing or stale validator pass link (verify: file .github/workflows/ci.yml contains Validator pass link guard)
- [ ] C5: A BLOCKED round exits non-zero so it can never be recorded as PASS (verify: file scripts/validation/validate.py contains return 2)
- [ ] C6: The merge gate refuses to merge without a full pass record (verify: file scripts/gates/merge_gate.py contains NO MERGE)
- [ ] C7: Requirements come from the approved spec, and spec approval confirms them (verify: file docs/workflow.md contains from the approved spec)
- [ ] C8: Validation evidence lives on the Issue, never in git (verify: file .gitignore contains .agent/validation/issue-)
- [ ] C9: Every validator FAIL routes to In progress, including correction rounds (verify: file docs/board.md contains including correction rounds)
- [ ] C10: The retry bound is two consecutive validator fails, written once (verify: file docs/workflow.md contains two consecutive validator fails)
- [ ] C11: Post-merge validation runs on the merge commit before any In review (verify: file docs/workflow.md contains post-merge validation must run on the merge commit)
- [ ] C12: Every listed sub-issue is closed after the squash merge (verify: file .opencode/skills/pull-request/SKILL.md contains Close every sub-issue)
- [ ] C13: The acceptance word lives in the config next to the approval words (verify: file .agent/project-config.json contains acceptanceWord)
- [ ] C14: The owner vocabulary is defined once in the workflow document (verify: file docs/workflow.md contains Acceptance word for Done)
- [ ] C15: No literal status name or option ID outside the config, checked by CI (verify: file .github/workflows/ci.yml contains Doc drift guard)
- [ ] C16: Board hygiene is checked by script, with no open task issues on the board (verify: file scripts/sidebar/project.py contains no open task-labelled)
- [ ] C17: One command shows everything waiting on the owner with the next stage (verify: file scripts/sidebar/project.py contains waitingOnOwner)
- [ ] C18: Untaken drafts are listed with whether an Issue already names them (verify: file scripts/intake/list_drafts.py contains taken)
- [ ] C19: The validator brief names two comments by link and never reads the diff (verify: file .opencode/agents/validator.md contains does not read the branch diff)
- [ ] C20: The validation runs use locked dependencies throughout (verify: file scripts/validation/validate.py contains --locked)
- [ ] C21: Agent-authored drafts and chore branches have a documented path (verify: file docs/workflow.md contains chore/<slug>)
- [ ] C22: The branch guard accepts the maintenance branch form (verify: file .github/workflows/ci.yml contains chore/)
- [ ] C23: CI pins clippy warnings-denied and fmt, and covers Windows (verify: file .github/workflows/ci.yml contains cargo fmt --check)
- [ ] C24: The app has a capture mode that writes assertable frames per level (verify: file crates/universe-app/src/main.rs contains CAPTURE-OK)
- [ ] C25: The headless journey check is healthy on the tested commit (verify: contains VERIFY-OK in verify.log)
- [ ] C26: The workspace test suite is green on the tested commit (verify: contains test result: ok in cargo-test.log)
- [ ] C27: The repeat run is deterministic on the tested commit (verify: contains VERIFY-OK in edge-verify-repeat.log)
