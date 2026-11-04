# #1085 lightweight rendering investigation: index

**Outcome:**
- **Prototype:** VSync-on frame delivery with one frame in flight (`inflight1`). It is **promising** under the Spec v1 rules, but **not recommended for rollout**.
- **Follow-up:** the adaptive experiment the owner chose (A) failed.
- **Decision:** the owner then chose **B1**: stop, keep the current frame delivery, and keep the developer switch and the evidence (Telegram, 2026-11-04 10:55; #1085 comment 6628890802).

| step | issue | evidence | finding |
|---|---|---|---|
| T1 | #1148 | `t1/README.md` | GPU pass timers per route scenario; the frame-work wait split |
| T2 | #1149 | `t2/README.md` | VSync-on frame delivery waits about 32 ms of a ~40 ms frame; the GPU needs 12-15 ms; the atmosphere tier costs nothing measurable; shadows +7 ms and bloom +3 ms (both off by default). The candidate switched to frame delivery. |
| T3 | #1150 | `t3/README.md` | switch `ASTROLITH_PROTOTYPE` / route `prototype`; only `inflight1` changes delivery (ground 39.86 → 27.07 ms) |
| T4 | #1151 | `t4/README.md` | matched pairs: VSync off rejected; VSync on promising (Low 5/8 scenes, Normal universe and orbit); rollout numbers not met; the VSync-on baseline has two states |
| T5 | #1152 | `t5/README.md`, **`t5/REPORT.md`** | pictures identical within noise; in the real window, save, restart and Reset unchanged; Auto check; criteria map; the owner report |
| T6 | #1185 | `t6/README.md` | `adaptive` built; the 40 ms lock is one-way, so no switching controller reaches the `inflight1` numbers |
| T7 | #1186 | none | skipped by owner decision B1 |

**What stays in the code (developer only, never a setting):**
- the prototype switch (`crates/universe-app/src/prototype.rs`, `adaptive.rs`);
- GPU pass timers for routes (`crates/universe-render/src/route_gpu.rs`);
- `prototype=` in run lines and manifests.

Without the switch, the game runs exactly the #819 pacing.

**Requirements for validation:** `requirements.md`.
