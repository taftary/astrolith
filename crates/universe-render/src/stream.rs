//! Background preview generation off the frame thread (#152).
//!
//! Everything here is `pub(crate)`: only this crate uses it. The headless
//! proof generates previews synchronously in `universe-core`; the window
//! instead spawns one task per previewed marker on the compute pool, polls
//! completions without blocking, and merges them into the preview cache in
//! marker order, so the merged set equals the synchronous result. A
//! [`StreamLedger`](universe_core::stream::StreamLedger) mirror tracks the
//! same working-set bound the headless proof asserts. Buffers are reused
//! across frames (`E-HOT-NOALLOC`).

use crate::{PreviewCache, Universe};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future::block_on};
use universe_core::r#gen::Generated;
use universe_core::preview::append_cell_preview_set;
use universe_core::stream::StreamLedger;
use universe_core::streamed::preview_cell;

/// Pending background generations plus reused buffers and the bound mirror.
#[derive(Resource, Default)]
pub(crate) struct StreamTasks {
    pending: Vec<(u32, Task<Generated>)>,
    wanted: Vec<u32>,
    completed: Vec<(u32, Generated)>,
    scratch: Vec<u32>,
    chain: Vec<u32>,
    pub(crate) ledger: StreamLedger,
}

/// Brings the blocks near the camera into the live L1 and L5 cells
/// (#1104 T6). Marks the universe changed only when markers changed, so a
/// still camera costs nothing downstream.
pub(crate) fn stream_blocks(mut universe: ResMut<Universe>) {
    if universe.bypass_change_detection().stream_blocks() {
        universe.set_changed();
    }
}

/// Keeps the preview cache in line with the camera without generating on
/// the frame thread: the wanted set is computed inline (pure and cheap),
/// missing markers spawn background tasks, finished tasks merge in marker
/// order, and the ledger mirrors the bound.
pub(crate) fn sync_previews(
    universe: Res<Universe>,
    nav: Option<Res<crate::Navigation>>,
    mut previews: ResMut<PreviewCache>,
    mut tasks: ResMut<StreamTasks>,
) {
    // #1104 T6: the aimed marker always previews once large enough.
    let pin = nav.and_then(|nav| nav.target);
    if previews.pin != pin {
        previews.pin = pin;
    }
    let StreamTasks {
        pending,
        wanted,
        completed,
        scratch,
        chain,
        ledger,
    } = &mut *tasks;
    wanted.clear();
    if universe.path.can_open() {
        append_cell_preview_set(
            wanted,
            universe.path.offset(),
            &universe.open,
            universe.marker_radius(),
            previews.pin,
        );
    }
    pending.retain(|(marker, _)| wanted.contains(marker));
    let pool = AsyncComputeTaskPool::get();
    let root = universe.root.0;
    for &marker in wanted.iter() {
        let cached = previews.entries().iter().any(|(held, _)| *held == marker);
        let tasked = pending.iter().any(|(held, _)| *held == marker);
        if cached || tasked {
            continue;
        }
        let mut chain = universe.path.indices();
        chain.push(marker);
        // #1104 T6: a streamed child shows the blocks near the camera.
        let camera = universe_core::streamed::child_camera(&universe, marker);
        pending.push((
            marker,
            pool.spawn(async move { preview_cell(root, &chain, camera) }),
        ));
    }
    completed.clear();
    let mut index = 0;
    while index < pending.len() {
        let finished = pending
            .get(index)
            .is_some_and(|(_, task)| task.is_finished());
        if finished {
            let (marker, task) = pending.remove(index);
            completed.push((marker, block_on(task)));
        } else {
            index += 1;
        }
    }
    previews.merge(&universe, wanted, completed);
    // Ledger mirror (#152, #742): every open-chain prefix at distance
    // zero (the camera sits inside them) plus one chain per wanted marker
    // at its camera gap, marked at one tick, then capped.
    let offset = universe.path.offset();
    ledger.begin_sync();
    scratch.clear();
    scratch.extend(
        universe
            .path
            .chain()
            .iter()
            .filter(|opened| !opened.anonymous)
            .map(|opened| opened.marker),
    );
    let mut end = 0;
    while end <= scratch.len() {
        if let Some(prefix) = scratch.get(..end) {
            ledger.touch(root, prefix, 0.0);
        }
        end += 1;
    }
    for &marker in wanted.iter() {
        chain.clear();
        chain.extend(scratch.iter().copied());
        chain.push(marker);
        let distance = universe
            .open
            .points
            .get(marker as usize)
            .map(|point| {
                let at = point.portal_position();
                ((offset[0] - at[0]).powi(2)
                    + (offset[1] - at[1]).powi(2)
                    + (offset[2] - at[2]).powi(2))
                .sqrt()
            })
            .unwrap_or(f64::INFINITY);
        ledger.touch(root, chain.as_slice(), distance);
    }
    ledger.enforce_cap();
}
