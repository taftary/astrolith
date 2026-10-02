---
name: wait-checks
description: Load when waiting for PR checks to finish (part of the pull-request flow). Use to wait with visible progress and timeout, in background with notification.
---

# Wait Checks

Reusable wait for PR automatic checks. Designed for non-blocking use: start in background, continue other work, get notified. Moves no status; the caller manages the move.

## Inputs

- PR number or head commit SHA.
- Timeout seconds and poll interval seconds (defaults: 600 / 15).

## Steps

1. Start the wait (blocking call with progress to stderr, JSON to stdout):
   - `python scripts/sidebar/checks.py wait --pr <N> --timeout 600 --interval 15`
   - Or by SHA: `python scripts/sidebar/checks.py wait --sha <full-sha> --timeout 60 --interval 5`
2. For non-blocking use, run it as a background task and continue other work; read the JSON result when notified (`ok`, `state`: `success` / `failure` / `timeout`, `runs`, `failed`).
3. On `success` (exit 0) continue the PR flow. On `failure` (exit 1) name the failed check from `failed`. On `timeout` stop and retry with a longer timeout or report.
4. For PR-body checks with quotes, always read from a file:
   - `python scripts/sidebar/development.py check-body --body-file <path>`

## Outputs

- JSON result with `state` and `runs` for the head SHA.
- Progress lines on stderr (one per poll with timestamp).

## Done criteria

- Wait returns `success` for green PRs, `failure` naming the failed check for red PRs, `timeout` when the limit is hit.
- Quoted PR bodies checked via `--body-file` without shell-quoting failure.
