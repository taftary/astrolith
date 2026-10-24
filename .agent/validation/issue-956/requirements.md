# Requirements — Issue #956 (Scale HUD)

From Spec v2 (https://github.com/taftary/astrolith/issues/956#issuecomment-6382667017), approved in https://github.com/taftary/astrolith/issues/956#issuecomment-6382669320.

- [ ] C1: Fresh start shows nothing in the bottom-left corner (AC1) (verify: default-settings screens shot shows no scale-hud text)
- [ ] C2: Settings Display Scene has a Scale HUD switch, off by default (AC2) (verify: settings file carries scale_hud key default off and Settings screen lists it)
- [ ] C3: Turning Scale HUD on shows the level, distance and scale lines, and at the planet the height band and height above ground (AC3) (verify: screens shot with switch on shows level/distance/scale text)
- [ ] C4: Turning Scale HUD off hides all of those lines at once, immediately, with no restart, including pull and ground-memory lines (AC4) (verify: screens shot with switch off shows no scale-hud text)
- [ ] C5: The choice is kept after closing and reopening the game, and Reset Display puts it back to hidden (AC5) (verify: settings file round-trips scale_hud and reset restores default)
- [ ] C6: Help then Navigation carries the new sentence naming the switch (AC6) (verify: guides screen Navigation text contains Scale HUD sentence)
- [ ] C7: Nothing else on screen changes (AC7) (verify: screens diff outside scale-hud and guides text is empty)

Owner-test-only (in the Spec v2 test plan, presented for owner testing after merge, not machine-checked here): fresh start empty, switch on shows lines, dive follows height, switch off empties at once, choice kept across restart, reset hides again, guides sentence present (owner words: "Hideable via Settings. Hidden by default."). Covered mechanically by C1-C6 above.
