# .agent/

Machine config and evidence. Tracked files are the config and the
confirmed requirements; untracked files are scratch and timestamped
evidence whose record lives on the Issue.

- `project-config.json` (tracked): project IDs, Status option IDs, and
  lifecycle mapping. Skills and scripts read Status names and IDs from
  here, never hardcoded.
- `backfill/` (mixed): backfill checkpoints and dry-run outputs. Dry-run
  CSV/JSON snapshots are tracked; `backfill-before-*` and `checkpoint.json`
  are untracked working files.
- `validation/` (mixed): per-issue validation state. `requirements.md`
  directly under `issue-<n>/` is tracked (the confirmed spec restatement);
  timestamped `20*/` run directories (logs, reports) are untracked, ignored
  by `.gitignore`; the Issue comment is the record.
