# Board

Shared reference for all skills. Every skill ends by moving the board status per this file and recording its output on the Issue.

Source of truth for the process is `workflow.md` in this directory. This file is a reference, not a skill.

Canonical config: `.agent/project-config.json` (project IDs, Status option IDs, lifecycle mapping). Never hardcode Status names or IDs in skills or scripts; read them from that file. The sidebar scripts in `scripts/sidebar/` and the backfill in `scripts/backfill/` already do.

## IDs (verified 2026-10-01, renamed Inbox->Todo, Working->In progress, Ready to test->In review; IDs preserved)

- Project "Astrolith": number `3` (owner `taftary`), node ID `PVT_kwHOCpw4Tc4BlUJb`, https://github.com/users/taftary/projects/3
- `FIELD_ID` (Status): `PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE`
- `OPTION_TODO_ID`: `9b85cf8e` (display `Todo`)
- `OPTION_NEEDS_YOUR_ANSWER_ID`: `8cca15ae` (display `Needs your answer`; this is the option the UI truncates as "Needs your ...")
- `OPTION_IN_PROGRESS_ID`: `37b951ef` (display `In progress`)
- `OPTION_IN_REVIEW_ID`: `3d5f8e66` (display `In review`)
- `OPTION_NEEDS_CORRECTION_ID`: `bfd90995`
- `OPTION_DONE_ID`: `5a609255`

Discover them with:

```sh
gh project list --owner taftary
gh project field-list 3 --owner taftary
gh api graphql -f query='query { node(id: "PVT_kwHOCpw4Tc4BlUJb") { ... on ProjectV2 { field(name: "Status") { ... on ProjectV2SingleSelectField { options { id name } } } } } }'
```

Only parent Issues (one per notion, feedback item, or bug) are tracked on the board. Sub-issues carry only the label `task` and never reach the board (auto-add filter is `is:issue is:open label:notion,feedback,bug`). Sub-issues that appear on the board arrived via the built-in "Auto-add sub-issues to project" workflow, which is currently enabled and must be disabled in project settings; until then, flag them in the backfill instead of silently keeping them.

## Status lifecycle (transitions are mandatory, verified, and validator-gated)

Update Status at the moment of the transition, not at the end of the task. Use `scripts/sidebar/project.py set-status --issue N --status "<name>"`: it reads the current value first (`already set` when correct), adds the issue to the project when missing, retries once, re-reads to verify, and never fails silently.

| Event | Status to set |
|---|---|
| Issue picked up / work started | In progress |
| Branch created and commits pushed | In progress |
| Validator verdict = FAIL (any attempt) | In progress |
| Validator verdict = PASS and PR opened | In review |
| Blocked, validator BLOCKED, or owner decision/input needed | Needs your answer |
| PR merged AND post-merge validation passes (owner test still pending) | In review |
| Work paused or abandoned | Back to Todo |
| Owner accepts the merged result (or explicitly abandons) | Done (Issue closed) |

Rules:

- Never move an issue to "In review" or "Done" without a validator PASS on record for the exact commit (marker `<!-- validator:pass sha=<full-sha> -->` where the SHA equals the PR head SHA; stale passes from older rounds do not count).
- A merged PR moves the item to "In review", never to "Done". The parent Issue is never closed automatically.
- Every status move: read current value, set only when different, re-read to verify, retry once, then report the exact error. `project.py set-status` does all of this; do not call `gh project item-edit` directly from skills.

## Status moves

Each command moves the parent Issue's project item to the named status. `ITEM_ID` is the project item ID for the parent Issue (from the `get` subcommand below), not the Issue number. Prefer the script over raw `item-edit`:

```sh
python scripts/sidebar/project.py get --issue N
python scripts/sidebar/project.py set-status --issue N --status "Todo"
python scripts/sidebar/project.py set-status --issue N --status "Needs your answer"
python scripts/sidebar/project.py set-status --issue N --status "In progress"
python scripts/sidebar/project.py set-status --issue N --status "In review"
python scripts/sidebar/project.py set-status --issue N --status "Needs correction"
python scripts/sidebar/project.py set-status --issue N --status "Done"
```

Raw equivalents (kept for reference; `item-edit` takes the project node ID `PVT_kwHOCpw4Tc4BlUJb`, not the project number):

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 9b85cf8e
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 8cca15ae
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 37b951ef
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 3d5f8e66
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id bfd90995
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 5a609255
```

Status meanings per `workflow.md`: `Needs your answer` and `In review` (merged, awaiting owner test) are the owner's turn; every other status is the AI's. A merged PR moves the item to `In review`, never to `Done`. The parent Issue is never closed automatically.

## Sub-issue attach recipe

Sub-issues carry only the label `task` and never reach the board.

1. Create the sub-issue (label `task` only):

```sh
gh issue create --title "<title>" --body "<done-criteria>" --label task
```

Note the new sub-issue number `SUB_NUMBER`.

2. Or attach with the relationships tool (warns on circular dependencies):

```sh
python scripts/sidebar/relationships.py add-sub --parent PARENT_NUMBER --sub SUB_NUMBER
python scripts/sidebar/relationships.py list --issue PARENT_NUMBER
```

Manual fallback (numeric GraphQL `id`, not the number):

```sh
gh api repos/{owner}/{repo}/issues/SUB_NUMBER --jq .id
```

3. Attach it as a sub-issue of parent `PARENT_NUMBER` (`sub_issue_id` must be a JSON number, not a string):

```sh
$id = gh api repos/{owner}/{repo}/issues/SUB_NUMBER --jq .id
'{ "sub_issue_id": ' + $id + ' }' | gh api repos/{owner}/{repo}/issues/PARENT_NUMBER/sub_issues -X POST --input -
```

Repeat for each task. Then verify none of the sub-issues is on the board (they carry only `task`, so the auto-add filter excludes them).

### Fallback

If the sub-issues API is unavailable, list the tasks in the parent Issue body instead:

```md
- [ ] #N
```

One line per sub-issue number. Replace with real sub-issue links once the API works.
