# scripts/

Repo tooling (Python). IDs come from `.agent/project-config.json`; never
hardcode Status names or option IDs, read them from that file.

- `backfill/`: one-shot board backfill (`backfill.py`) used when the
  project board needs rebuilding from Issue state.
- `gates/`: executable workflow gates called by skills instead of
  re-describing rules in prose (`spec_gate.py`, `merge_gate.py`,
  `done_gate.py`, `doc_drift_gate.py`, plus `compare_verdict.py` and its
  fixtures).
- `intake/`: drafts intake listing (`list_drafts.py`) for the start of a
  session.
- `sidebar/`: project sidebar operations: status moves (`project.py`),
  branch and PR helpers (`development.py`), sub-issue relationships
  (`relationships.py`), milestone, notification, and check helpers.
- `validation/`: runtime validation (`validate.py`: test suite plus the
  headless `--verify`/`--capture` probes) and file comparison
  (`compare_files.py`).
