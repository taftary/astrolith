# Validator Capabilities

| capability | how proven | Issue | date |
| read a rendered file on GitHub at a commit SHA | preflight: raw file URL + contents API at commit f60bf56 (blob SHA from contents API is not usable as ref) | #30 | 2026-10-01 |
| run the headless app check (`cargo run -p universe-app -- --verify`: exit code, `VERIFY-OK`, per-line PASS, two runs byte-identical) and `cargo test --locked --workspace` locally | preflight on main ba04b8f: rc=0, VERIFY-OK, 68 tests green, identical repeat (https://github.com/taftary/astrolith/issues/63#issuecomment-5940228312) | #63 | 2026-10-01 |
| poll GitHub check-runs / actions runs for a commit SHA and distinguish success vs failure, plus read a PR body containing double quotes from file content without shell-quoting failure (background concurrent fetch proven) | preflight: PR #61 head 176799a -> ci completed/success vs 91ef7cc -> ci completed/failure; PR #61 body len 2355 with quotes checked via regex; concurrent fetches 282ms/193ms (https://github.com/taftary/astrolith/issues/62#issuecomment-5947441919) | #62 | 2026-10-02 |
| run python repo scripts locally (`scripts/sidebar/project.py get --issue N`, `scripts/validation/validate.py --help`) as the tool class the new gate scripts belong to | preflight on main eaa71da: project.py get ok, validate.py usage printed (bare `python --version` denied by validator sandbox allowlist; script-path invocations work) | #76 | 2026-10-02 |
| run the app capture mode (`cargo run --locked -p universe-app -- --capture <dir>`: exit 0, `CAPTURE-OK`, 11 level snapshots + 11 frame plots + manifest, two runs byte-identical) | proven on feat/76-workflow-gates: CAPTURE-OK files=23 twice, 23/23 files byte-identical across runs | #76 | 2026-10-02 |

Later notions check only what is new; capabilities already proven here are not re-proven.
