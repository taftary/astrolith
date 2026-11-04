# #1158 deterministic reproductions (probes, not committed to code)

Each probe was applied to the test code at `fix/1158-deterministic-render-tests`, run 5 times with `cargo test --locked -p universe-render --lib <test>`, and then reverted.

| test | probe | before the fix | after the fix |
|---|---|---|---|
| `narrow_flow_lists_first_then_details_with_back` | force `panel_wheel_scroll.before(sync_panel_scroll)` in `PanelPlugin` (one of the two orders the executor may pick) | fails 5/5 ("wheel lines scroll the content": the clamp resets to 0 on an unmeasured layout) | passes 5/5 in both orders (test gives the content a measured size) |
| `narrow_flow…` | the other order, `panel_wheel_scroll.after(sync_panel_scroll)` | passes 5/5 | passes 5/5 |
| `narrow_flow…` | clock step 0, 100, 2000, 100000, 500000 or 2000000 µs | passes (the clock is not the cause) | — |
| `touch_walks_jumps_and_switches_camera_in_normal_mode` | without the built-ground wait, the background ground build lands on the frame after the jump (`ground_pending = true` between the jump press and release frames) | fails 5/5 ("Jump leaves the ground") | not reachable: the test now starts on the built ground, so `ground_pending` is already false |
| `touch_walks…` | the same build landing on the jump-press frame | passes 5/5 (the placement runs before the jump) | — |

The other two tests are recorded in the #1158 implementation comment:
- `shift_digit…` failed on every run on the fixed clock;
- `behind_follow…` failed 20/20 on the real clock with frames of about 2 ms.
