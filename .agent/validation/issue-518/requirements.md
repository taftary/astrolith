# Requirements — #518 Settings/Guides panel pages overlap when switching selection

Source: Spec v2 (Issue comment 6112693917, owner `approved` 6112724696).
Criteria C1–C5 carry the spec's own numbers; C6 is the owner expectation
that needs eyes on the real window. Each criterion is in the owner's words;
a `(verify: ...)` probe is given where automation can check it, and a
`... ok` probe only matches a test that passed.

Status: CONFIRMED (derived from the approved Spec v2).

- [ ] C1: Settings: clicking each left item (Display, Scene, Window, Controls, Flight, Keys, Views, Accessibility, Developer leaf pages) shows that page alone from its top; no leftover content from the previous page above or below it. (verify: contains only_the_selected_section_keeps_layout_space ... ok in cargo-test.log)
- [ ] C2: Guides: clicking each left item (About, What is Astrolith, The scale ladder, Help, Getting started, Navigation, Free flight, Saved views) shows that page alone from its top. (verify: contains only_the_selected_page_keeps_layout_space ... ok in cargo-test.log)
- [ ] C3: Switching pages always jumps back to the top, even if the previous page was scrolled down. (verify: contains new_selection_resets_the_scroll_to_top ... ok in cargo-test.log)
- [ ] C4: A visible vertical scroll bar sits on the right content area; mouse wheel and dragging it both scroll long pages; the left menu does not move while content scrolls. (verify: contains sync_places_the_thumb_on_live_layout ... ok in cargo-test.log)
- [ ] C5: Nothing else changes: values, keys, saved views, and narrow list/detail with Back keep working. (verify: contains narrow_flow_lists_first_then_details_with_back ... ok in cargo-test.log)
- [ ] C6: On the real window, every page opens at its top with no trace of the previous page, the thumb is visible on long pages and follows wheel and drag, and the left menu stays put while the right side scrolls. These need eyes on the real window.
