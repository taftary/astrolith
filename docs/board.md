# Board

Shared reference for all skills. Every skill ends by moving the board status per this file and recording its output on the Issue.

Source of truth for the process is `workflow.md` in this directory. This file is a reference, not a skill.

## IDs (filled in Phase 4 task 4.5, verified 2026-10-01)

- Project "Astrolith": number `3` (owner `taftary`), node ID `PVT_kwHOCpw4Tc4BlUJb`, https://github.com/users/taftary/projects/3
- `FIELD_ID` (Status): `PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE`
- `OPTION_INBOX_ID`: `9b85cf8e`
- `OPTION_NEEDS_YOUR_ANSWER_ID`: `8cca15ae`
- `OPTION_WORKING_ID`: `37b951ef`
- `OPTION_READY_TO_TEST_ID`: `3d5f8e66`
- `OPTION_NEEDS_CORRECTION_ID`: `bfd90995`
- `OPTION_DONE_ID`: `5a609255`

Discover them with:

```sh
gh project list --owner taftary
gh project field-list 3 --owner taftary
gh project item-list 3 --owner taftary
```

Only parent Issues (one per notion, feedback item, or bug) are tracked on the board. Sub-issues carry only the label `task` and never reach the board (auto-add filter is `is:issue is:open label:notion,feedback,bug`). The built-in "Auto-add sub-issues to project" workflow was deleted via API during Phase 4, so adding a parent never pulls its sub-issues in.

## Status moves

Each command moves the parent Issue's project item to the named status. `ITEM_ID` is the project item ID for the parent Issue (from `gh project item-list`), not the Issue number. `item-edit` takes the project node ID (`PVT_kwHOCpw4Tc4BlUJb`), not the project number (`item-add`, `item-list`, `item-delete` take the number `3`).

### Inbox

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 9b85cf8e
```

### Needs your answer

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 8cca15ae
```

### Working

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 37b951ef
```

### Ready to test

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 3d5f8e66
```

### Needs correction

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id bfd90995
```

### Done

```sh
gh project item-edit --project-id PVT_kwHOCpw4Tc4BlUJb --id ITEM_ID --field-id PVTSSF_lAHOCpw4Tc4BlUJbzhkBfuE --single-select-option-id 5a609255
```

Status meanings per `workflow.md`: `Needs your answer` and `Ready to test` are the owner's turn; every other status is the AI's. A merged PR moves the item to `Ready to test`, never to `Done`. The parent Issue is never closed automatically.

## Sub-issue attach recipe

Sub-issues carry only the label `task` and never reach the board.

1. Create the sub-issue (label `task` only):

```sh
gh issue create --title "<title>" --body "<done-criteria>" --label task
```

Note the new sub-issue number `SUB_NUMBER`.

2. Take its numeric GraphQL `id` (not its number):

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
