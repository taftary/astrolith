# 0023 — Bundled UI fonts (Inter and Lucide)

- Status: proposed
- Date: 2026-10-11
- Issue: #473 (Spec v3 and v4 approved)

## Context

Every `Text` node used Bevy's `default_font`, an ASCII-only subset of
FiraMono. It has no icon glyphs, so the menu button showed the letter "M",
close showed "X" and Back showed "<". The approved #473 redesign asks for
smaller text in Inter, with Lucide icons, both shipped with the app and
under open licences.

## Decision

- `crates/universe-render/assets/fonts/` holds the font files and their
  licence files:

  | file | source | licence | SHA-256 |
  |---|---|---|---|
  | `Inter-Regular.ttf` | rsms/inter release v4.1, `Inter-4.1.zip` → `extras/ttf/` | SIL OFL 1.1 (`Inter-LICENSE.txt`) | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
  | `Inter-Medium.ttf` | same | same | `97ad806f526e41546d46365bb3a393145f75b7b1568913db74549ad8b8dba872` |
  | `lucide.ttf` | npm `lucide-static` 1.55.0, `font/lucide.ttf` | ISC (`Lucide-LICENSE.txt`) | `d4025263e2a86c24b972669e9596c0ace02db50cd3f222ef64a6563e5772695e` |

- The fonts are embedded with `include_bytes!` and added to `Assets<Font>`
  in `PreStartup` (`fonts.rs`, `UiFonts`). Nothing is read from disk at run
  time, so `--screens` and normal play behave the same from any folder.
- Each screen text carries a `TextRole` (Title, Body, ListItem, Note,
  Icon). One system sets its font and size from `tokens.rs`, so spawn
  sites never pass font handles around.
- Icons are named once (`fonts::Icon`), with their codepoints taken from
  `lucide-static` 1.55.0 `font/codepoints.json`.
- The HUD readout and the portal labels keep `default_font`, because they
  belong to the universe view, not the screens. `default_font` stays
  enabled.

## Consequences

- The binary grows by about 1.7 MB (three TTF files).
- Updating a font means replacing the file, recording the new source and
  SHA-256 here, and re-checking the icon codepoints.
- No new crate dependency and no version pin change (`E-PIN-ADR` not
  touched).
