# Board

Shared reference for all skills. Every skill ends by moving the board status per this file and recording its output on the Issue.

Source of truth for the process is `workflow.md` in this directory. This file is a reference, not a skill.

## Placeholders (fill in Phase 4 task 4.5)

> TODO (Phase 4 task 4.5): replace every placeholder below with the real IDs, then verify one status move command works from this file.

- `PROJECT_NUMBER`: GitHub Project number for "Astrolith" (owner `taftary`).
- `FIELD_ID`: node ID of the `Status` single-select field.
- `OPTION_INBOX_ID`: option ID for `Inbox`.
- `OPTION_NEEDS_YOUR_ANSWER_ID`: option ID for `Needs your answer`.
- `OPTION_WORKING_ID`: option ID for `Working`.
- `OPTION_READY_TO_TEST_ID`: option ID for `Ready to test`.
- `OPTION_NEEDS_CORRECTION_ID`: option ID for `Needs correction`.
- `OPTION_DONE_ID`: option ID for `Done`.

Discover them with:

```sh
gh project list --owner taftary
gh project field-list PROJECT_NUMBER --owner taftary
gh project item-list PROJECT_NUMBER --owner taftary
```

Only parent Issues (one per notion, feedback item, or bug) are tracked on the board. Sub-issues carry only the label `task` and never reach the board (auto-add filter is `is:issue is:open label:notion,feedback,bug`).

## Status moves

Each command moves the parent Issue's project item to the named status. `ITEM_ID` is the project item ID for the parent Issue (from `gh project item-list`), not the Issue number.

### Inbox

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_INBOX_ID
```

### Needs your answer

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_NEEDS_YOUR_ANSWER_ID
```

### Working

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_WORKING_ID
```

### Ready to test

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_READY_TO_TEST_ID
```

### Needs correction

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_NEEDS_CORRECTION_ID
```

### Done

```sh
gh project item-edit --project-id PROJECT_NUMBER --id ITEM_ID --field-id FIELD_ID --single-select-option-id OPTION_DONE_ID
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

3. Attach it as a sub-issue of parent `PARENT_NUMBER`:

```sh
gh api repos/{owner}/{repo}/issues/PARENT_NUMBER/sub_issues -f sub_issue_id=<id-from-step-2>
```

Repeat for each task. Then verify none of the sub-issues is on the board (they carry only `task`, so the auto-add filter excludes them).

### Fallback

If the sub-issues API is unavailable, list the tasks in the parent Issue body instead:

```md
- [ ] #N
```

One line per sub-issue number. Replace with real sub-issue links once the API works.
