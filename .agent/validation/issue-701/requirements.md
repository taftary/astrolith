# Requirements (CONFIRMED) - issue #701

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/701#issuecomment-6186019443,
owner approval https://github.com/taftary/astrolith/issues/701#issuecomment-6186315326),
with the owner decisions Q1 wheel only, Q2 scroll-to-selected yes, Q3 list
thumb yes. How the scrolled list looks on the real window is judged by the
main agent's `--screens` pictures and the owner test; the validator takes
no pictures (#533).

- [ ] C1: Wheeling over the Guides or Settings list scrolls its rows; wheeling over the content still scrolls the content (AC1) (verify: contains wheel_over_the_list_scrolls_rows_not_content ... ok in cargo-test.log)
- [ ] C2: Selecting a row scrolls the list so the selected row is visible, including opening Guides straight at Help > Planets (AC2) (verify: contains selecting_a_row_reveals_it_in_the_list ... ok in cargo-test.log)
- [ ] C3: The list shows a scrollbar thumb matching the content side only while rows overflow; lists that fit show none (AC3) (verify: contains list_thumb_shows_only_on_overflow ... ok in cargo-test.log)
- [ ] C4: Narrow windows behave the same, the list step scrolls (AC4) (verify: contains narrow_list_scrolls_like_the_wide_list ... ok in cargo-test.log)
- [ ] C5: The guides-planets shot setup selects Planets and reveals its row (AC5) (verify: contains guides_planets_shot_reveals_its_row ... ok in cargo-test.log)
- [ ] C6: Style row heights and all chrome are unchanged (AC5) (verify: contains list_rows_keep_the_shared_button_height ... ok in cargo-test.log)
