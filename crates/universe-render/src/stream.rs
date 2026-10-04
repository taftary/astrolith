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
use universe_core::nest::generate_cell;
use universe_core::preview::append_preview_set;
use universe_core::stream::StreamLedger;

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

/// Keeps the preview cache in line with the camera without generating on
/// the frame thread: the wanted set is computed inline (pure and cheap),
/// missing markers spawn background tasks, finished tasks merge in marker
/// order, and the ledger mirrors the bound.
pub(crate) fn sync_previews(
    universe: Res<Universe>,
    mut previews: ResMut<PreviewCache>,
    mut tasks: ResMut<StreamTasks>,
) {
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
        append_preview_set(
            wanted,
            universe.path.offset(),
            &universe.open.points,
            universe.marker_radius(),
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
        pending.push((
            marker,
            pool.spawn(async move { generate_cell(root, &chain) }),
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
    // Ledger mirror (#152): every open-chain prefix plus one chain per
    // wanted marker, marked at one tick, then capped.
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
            ledger.touch(root, prefix);
        }
        end += 1;
    }
    for &marker in wanted.iter() {
        chain.clear();
        chain.extend(scratch.iter().copied());
        chain.push(marker);
        ledger.touch(root, chain.as_slice());
    }
    ledger.enforce_cap();
}
