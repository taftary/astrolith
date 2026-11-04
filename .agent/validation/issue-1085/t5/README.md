# #1085 T5 (#1152): pictures, behaviour and owner decision package

The owner-facing report is **[REPORT.md](REPORT.md)**. This file holds the evidence and the criteria map.

**Build:** release `universe-app.exe` SHA-256 `b5df90ccc0137cd23fd1ed9c4cc4dfa6f353f6656446013578339cc86fc6e666`, from commit c92ccfd. That commit is the T4 code plus one change: picture runs (`--screens`) now honour `ASTROLITH_PROTOTYPE`, so pictures can be paired. Pacing code is unchanged.

**Checks:**
- `cargo clippy --workspace --all-targets -D warnings` is clean.
- `cargo test -p universe-app -p universe-render`: 37 + 1 + 285 tests pass.
- `--verify`: `VERIFY-OK`.

Same PC as T2-T4: Intel UHD 620, 60 Hz, Balanced on AC.

## Paired pictures (`pictures-base/`, `pictures-base2/`, `pictures-inflight1/`)

- **Shots:** `paired.shots`, 19 shots at 1600x900, in picture mode (Manual quality, shadows and bloom on: the game's picture look). Vulkan.
- **Runs:** the baseline twice (no switch) and once with `ASTROLITH_PROTOTYPE=inflight1`. The `inflight1` run printed `prototype inflight1 in force (developer switch)`, and every run ended `SCREENS-OK files=20`.
- **Comparison:** `tools/picture_diff.py` compares every picture. It reports the share of pixels that differ by more than 8/255, the mean absolute difference, and the largest channel difference (`picture-diff.md`):

| picture | vs pictures-base2: changed px / mean / max | vs pictures-inflight1: changed px / mean / max |
|---|---|---|
| body | 0.00% / 0.00 / 1 | 0.00% / 0.00 / 1 |
| dawn | 0.00% / 0.17 / 1 | 0.00% / 0.26 / 1 |
| day | 0.00% / 0.00 / 0 | 0.00% / 0.00 / 1 |
| descent-2000km | 0.00% / 0.00 / 1 | 0.00% / 0.00 / 1 |
| descent-200km | 0.00% / 0.00 / 1 | 0.00% / 0.00 / 1 |
| descent-200m | 0.00% / 0.00 / 0 | 0.00% / 0.00 / 0 |
| descent-20km | 0.00% / 0.00 / 1 | 0.00% / 0.00 / 1 |
| descent-2km | 0.00% / 0.00 / 0 | 0.00% / 0.00 / 1 |
| descent-2m | 0.00% / 0.00 / 0 | 0.00% / 0.00 / 0 |
| dusk | 0.00% / 0.89 / 2 | 0.00% / 1.49 / 3 |
| moonless | 0.00% / 0.04 / 1 | 0.00% / 0.11 / 1 |
| moonlit | 0.00% / 0.09 / 1 | 0.00% / 0.08 / 1 |
| orbit | 0.00% / 0.04 / 1 | 0.00% / 0.14 / 2 |
| selection | 1.45% / 0.26 / 108 | 1.91% / 0.37 / 106 |
| turn-000 | 0.00% / 0.01 / 1 | 0.00% / 0.22 / 1 |
| turn-090 | 0.00% / 0.00 / 1 | 0.00% / 0.06 / 1 |
| turn-180 | 0.00% / 0.00 / 1 | 0.00% / 0.01 / 1 |
| turn-270 | 0.00% / 0.00 / 1 | 0.00% / 0.00 / 1 |
| world-two | 0.00% / 0.08 / 1 | 0.00% / 0.69 / 2 |

**Reading:**
- Baseline against `inflight1` differs as little as baseline against baseline: at most 3/255 per channel, and no pixel moves by more than 8/255.
- The one exception is `selection`, and it is the same in both comparisons (1.45% vs 1.91% of pixels). It is run-to-run motion of the star-system markers, not the prototype.
- Nothing drawn differs, as expected from a change to *when* a frame is shown.

**Coverage:**
- Daylight ground (`day`) and dawn/dusk (`dawn`, `dusk`; local time 08:22 plus `time=` at lon 60).
- Moonlit and moonless night, and orbit.
- Star system with selection, and the second world (`world-two`).
- Rapid turns (`turn-000` to `turn-270`), and LOD changes / continuous descent (`descent-2000km` to `descent-2m`).
- First person (`body`): ground surface only. **The body is missing in both builds.** It is the known baseline defect, which the owner chose to leave as is (A, 2026-11-03, #1085 comment 6598361579).

**Motion** is measured, not filmed: frame intervals per frame for each scenario are in `../t4/*/route/frames.txt`. The T4 report covers turns, descent, ground, transitions and the 20-minute route. No video was recorded.

## Real-window check (`real-window/`)

- **Setup:** the real game window (`WGPU_BACKEND=dx12` with dxc so screen capture works, the same exe) with a disposable `APPDATA` profile. It was driven by real mouse clicks (`real-window/win.ps1`).
- **Your own settings:** the owner's `%APPDATA%\Astrolith\settings.toml` SHA-256 was `0085d820…` before and after.

| launch | switch | what was done | result |
|---|---|---|---|
| 1 | none | Settings > Performance: Quality Auto → Manual, Target 60 → 30 FPS; Scene: Exposure + twice | no prototype line in `l1-base-stderr.txt`; saved `exposure = 1.5625`, `quality_mode = manual`, `target_fps = 30` (`settings-after-l1.toml`) |
| 2 | `inflight1` | restart; checked Performance, Exposure − once | `prototype inflight1 in force` in stderr; Manual / 30 FPS shown after restart (`l2-perf.png`); saved `exposure = 1.25`, nothing else changed except `game_time` |
| 3 | none | restart; Performance > Reset Display | no prototype line; screen shows Auto / 60 FPS (`l3-after-reset2.png`); saved `exposure = 1`, `quality_mode = auto`, `target_fps = 60` |

None of the three saved files contains `prototype` or `inflight`. The switch cannot persist: it is read only from the environment or a route line, and `Settings` has no field for it (T3 tests). No errors or panics appeared in any stderr.

## Auto quality integration check (`auto-base/`, `auto-inflight1/`)

`auto-check.route`: Auto quality, VSync on, 30 s warm-up plus 60 s each of turns, ground and orbit. One pair (an integration check, not a matched comparison: Auto picks its own quality). Both manifests are `complete`, `settings_unchanged` and carry the right `prototype`.

| scenario | baseline mean / p95 | inflight1 mean / p95 | Auto's quality, baseline | Auto's quality, inflight1 |
|---|---|---|---|---|
| turns | 39.47 / 48.91 | 27.88 / 34.37 | terrain Low, scale 0.8 | terrain Low, mostly scale 1.0 |
| ground | 39.85 / 48.54 | 28.86 / 30.85 | terrain Low, scale 0.8 | terrain Low, then Normal, at 1.0 |
| orbit | 39.80 / 49.61 | 40.17 / 43.91 | terrain Low, scale 0.9-1.0 | terrain and markers Normal, at 1.0 |

With `inflight1`, Auto saw faster frames and raised quality. Even so, it stayed faster on turns and ground, and on orbit it held the same mean at Normal instead of Low. No frame was over 100 ms on either side.

## Spec v1 acceptance criteria → evidence

| # | criterion | evidence | status |
|---|---|---|---|
| 1 | Reproducibility | baseline 44d1d29 / exe hashes in every manifest (T2 `3ce17ca4…`, T3/T4 `e5447043…`, T5 `b5df90cc…`); fixtures in `scripts/validation/fixtures/profile/` and `t2/toggles/fixtures/`; batch scripts `t2/tools/`, `t4/batch-t4*.sh`, `t5/batch-auto.sh`; raw `frames.txt`/`route.txt`/telemetry per run; repeat steps in REPORT.md | met |
| 2 | Diagnosis | T1 (`t1/README.md`): GPU pass timers and wait split; T2 (`t2/README.md`): CPU / GPU / wait ranking, toggles for atmosphere, shadows, bloom, the cap; surface lighting, transparent and draw-submission cost from GPU passes; terrain build in readiness; unmeasured items named there | met |
| 3 | Bounded prototype | T3 (`t3/README.md`): `ASTROLITH_PROTOTYPE` / route `prototype` line, `inflight1` chosen as the only variant that changed delivery; normal launch stays baseline (T3 and here, launches 1 and 3); mobile risks in REPORT.md | met |
| 4 | Measured verdict | T4 (`t4/README.md`, `t4/t4-pairs.json`): 3 alternating pairs (rank-low 5), Spec v1 `classify` per scenario, sustained first/final five minutes, transitions; every failing scene listed | met: promising, rollout numbers not met |
| 5 | Visual and behaviour coverage | paired pictures above (19 scenes, no difference beyond noise); motion via T4 frame data; existing checks keep thresholds (`--verify`, tests); first-person body disclosed as a baseline defect (owner A) | met, with the disclosed body defect |
| 6 | Controls and default experience | real-window launches 1-3 above; Settings/Help untouched (no Settings or Guides code changed in #1085) | met |
| 7 | Owner decision package | REPORT.md: cost, visible trade-offs (none in pictures), resource/loading effects, target gap, next action, sample and repeat steps | met |
| 8 | Honest platform coverage | REPORT.md "Mobile": no device evidence (#1054) | met |
