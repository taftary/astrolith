//! Saving and bringing back views (#561 T3, owner answer Q8 A).
//!
//! [`save_view`] and [`recall_view`] are the one copy of the Save view and
//! Bring a view back rules: the keys (hold, then 1-8) call them from
//! `handle_input`, and the Views page's **Save here** and **Go** buttons
//! call them through [`view_buttons_click`], so touch reaches every saved
//! view without a keyboard. Everything here is `pub(crate)`.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation, SavedSlots, SavedView};
use crate::settings_ui::SettingsStore;
use bevy::prelude::*;
use universe_core::destination::{Destination, Way};

/// Marks a Save here button for one slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingSaveView(pub usize);

/// Marks a Go button for one slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingGoView(pub usize);

/// Saves the current view into `slot` (live slot and the settings store).
pub(crate) fn save_view(
    slot: usize,
    universe: &Universe,
    destination: Destination,
    flight: &Flight,
    slots: &mut SavedSlots,
    store: &mut SettingsStore,
) {
    if let Some(entry) = slots.slots.get_mut(slot)
        && let Some(stored) = store.0.views.get_mut(slot)
    {
        let saved = SavedView {
            offset: universe.path.offset(),
            destination: Some(destination),
            mode: flight.mode,
            yaw: flight.yaw,
            pitch: flight.pitch,
            free_forward: flight.free_forward,
            free_up: flight.free_up,
            step: flight.step,
        };
        *stored = Some(SettingsStore::view_pose(&saved));
        *entry = Some(saved);
    }
}

/// Brings back the view in `slot`, if one is saved. A saved destination
/// comes back when it runs through the open cell; a saved offset only
/// means something there.
pub(crate) fn recall_view(
    slot: usize,
    universe: &mut Universe,
    destination: &mut Destination,
    flight: &mut Flight,
    nav: &mut Navigation,
    slots: &SavedSlots,
) {
    let Some(view) = slots.slots.get(slot).copied().flatten() else {
        return;
    };
    universe.path.set_offset(view.offset);
    if let Some(saved) = view.destination
        && saved.on_the_way(&universe.path) != Way::Off
    {
        *destination = saved;
    }
    flight.mode = view.mode;
    flight.yaw = view.yaw;
    flight.pitch = view.pitch;
    flight.target_yaw = view.yaw;
    flight.target_pitch = view.pitch;
    flight.free_forward = view.free_forward;
    flight.free_up = view.free_up;
    if view.mode == FlightMode::Free {
        // The free pose is the view itself; the camera follows it.
        nav.forward = view.free_forward;
        nav.up = view.free_up;
    }
    flight.step = view.step;
}

/// The Views page's Save here and Go buttons.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn view_buttons_click(
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    mut flight: ResMut<Flight>,
    mut slots: ResMut<SavedSlots>,
    mut store: ResMut<SettingsStore>,
    saves: Query<(&Interaction, &SettingSaveView), Changed<Interaction>>,
    goes: Query<(&Interaction, &SettingGoView), Changed<Interaction>>,
) {
    let root = universe.root.0;
    let mut destination = match nav.destination {
        None => Destination::seeded(root, &universe.path.indices()),
        Some(held) => held.reconciled(root, &universe.path),
    };
    for (interaction, save) in &saves {
        if *interaction == Interaction::Pressed {
            save_view(
                save.0,
                &universe,
                destination,
                &flight,
                &mut slots,
                &mut store,
            );
        }
    }
    for (interaction, go) in &goes {
        if *interaction == Interaction::Pressed {
            recall_view(
                go.0,
                &mut universe,
                &mut destination,
                &mut flight,
                &mut nav,
                &slots,
            );
            nav.destination = Some(destination);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    fn press<T: Component>(app: &mut App, matches: impl Fn(&T) -> bool) {
        let entity = app
            .world_mut()
            .query::<(Entity, &T)>()
            .iter(app.world())
            .find(|(_, widget)| matches(widget))
            .map(|(entity, _)| entity)
            .expect("button exists");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    #[test]
    fn save_here_then_go_brings_the_view_back() {
        let mut app = headless_app();
        app.update();
        app.update();
        let here = app.world().resource::<Universe>().path.offset();
        press::<SettingSaveView>(&mut app, |b| b.0 == 0);
        assert!(app.world().resource::<SettingsStore>().0.views[0].is_some());
        assert!(app.world().resource::<SavedSlots>().slots[0].is_some());
        app.world_mut()
            .resource_mut::<Universe>()
            .path
            .set_offset([0.2, -0.1, 0.3]);
        press::<SettingGoView>(&mut app, |b| b.0 == 0);
        assert_eq!(app.world().resource::<Universe>().path.offset(), here);
    }
}
