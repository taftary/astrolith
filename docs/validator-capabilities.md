# Validator Capabilities

| capability | how proven | Issue | date |
| read a rendered file on GitHub at a commit SHA | preflight: raw file URL + contents API at commit f60bf56 (blob SHA from contents API is not usable as ref) | #30 | 2026-10-01 |
| run the headless app check (`cargo run -p universe-app -- --verify`: exit code, `VERIFY-OK`, per-line PASS, two runs byte-identical) and `cargo test --locked --workspace` locally | preflight on main ba04b8f: rc=0, VERIFY-OK, 68 tests green, identical repeat (https://github.com/taftary/astrolith/issues/63#issuecomment-5940228312) | #63 | 2026-10-01 |

Later notions check only what is new; capabilities already proven here are not re-proven.
