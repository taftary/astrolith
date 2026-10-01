# Known misses (regression log)

Every bug the validator missed gets a row here plus a check that would have caught it.
`validate.py` runs every check in this file on each future validation.

| date | issue | what was missed | which step was lacking | check added |
| --- | --- | --- | --- | --- |
| 2026-10-01 | — | Validator passed on CI-green without ever launching the app (Bevy `--verify` never run; logs never scanned). | runtime validation (no launch, no log scan, CI-green treated as success) | `cargo test --workspace` + `cargo run -p universe-app -- --verify` must exit 0 with `VERIFY-OK`; full stdout/stderr scanned for error/exception/traceback/panic/failed; any hit fails unless allowlisted here. |
| 2026-10-01 | — | Result compared against implementer's summary instead of the owner's words. | requirements validation (wrong source of truth) | requirements checklist is built from the issue body/comments verbatim first; each criterion cites its source; validator never marks MET from a summary claim without running the check itself. |
