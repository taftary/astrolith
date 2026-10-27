# Requirements — Issue #1029 (ground faces in debug builds)

From Spec v1 (https://github.com/taftary/astrolith/issues/1029#issuecomment-6435272395), approved in https://github.com/taftary/astrolith/issues/1029#issuecomment-6435312853.

- [ ] C1: In a debug build the ground faces are drawn, near and far, as in a release build, and the log shows no `failed to process shader` error (AC1, AC2) (verify: file .agent/validation/issue-1029/debug-run.txt contains shader errors: 0)
- [ ] C2: A test stops any shader module the game imports from using a name the shader library would refuse in debug builds (AC3) (verify: contains imported_shader_modules_use_no_names_naga_would_rename ... ok in cargo-test.log)
- [ ] C3: The release ground is unchanged; the rename keeps the maths (AC4) (verify: contains ground_shader_agrees_with_core ... ok in cargo-test.log)
- [ ] C4: The game's headless check still passes (verify: contains VERIFY-OK in verify.log)
