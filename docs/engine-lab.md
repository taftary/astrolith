# Engine lab

A development program for the game's universe (#640, renamed in #654,
ADR 0029). It shows any level of the game from L1 to
L10, a planet with its moons and its L11-L14 shells included, drawn with
the game's own code. It is never part of the game and is never shipped to
players. It runs on Windows, Linux and macOS desktops only.

## Run

```
cargo run -p engine-lab
cargo run -p engine-lab -- --screens <folder>
```

The second form takes every picture into the folder (it must exist), writes
`screens.txt`, prints `SCREENS-OK files=<n> dir=<folder>`, and exits 0.
A wrong command line or a missing folder exits 2.

On Windows the lab uses Vulkan by default. With egui, DirectX 12 loses the
device on the dev PC's Intel GPU (ADR 0029). Set `WGPU_BACKEND=dx12` to try
DirectX 12 anyway.

## Window

- **Menu bar:**
  - File: Screenshot (F12), Quit.
  - View: Wireframe, Normals, LOD depth view, Stats overlay, Grid, Axes.
  - Camera: Orbit / Free-fly (G), Home, and the seven preset views.
  - Shaders: Planet squares (ticked when on) and Debug view (#665).
  - Help: About.
- **Nodes** (left):
  - On a planet: Planet, one line per moon, then Camera, Light, Grid and Axes.
  - At every level, "Shaders" with one line per game shader under it.
  - With a moon on show: that moon, then the tools.
  - Above the planet (L1-L9): the level's own objects by their game names,
    then the tools.
  - Click a line to select it.
- **Tools and options** (right):
  - the seed and path fields with Regenerate;
  - Parent, Child and Show selected (F), with the level and path on show;
  - Previous, Next and a slider over the star's planets ("Planet i of N"),
    on a planet only;
  - the selected node's values in real metres (the Planet node also
    shows the generator recipe: type, mass, gravity, age, surface
    temperature, air, plate regime and moon kinds, #685);
  - the Planet (or moon on show) node's debug toggles, with the splits per
    LOD depth when the LOD depth view is on.
  - A wrong seed, path or move shows a red message and keeps the view.
- **Status bar** (bottom): the seed, the path, "Planet i of N", the level
  on show and the last saved screenshot.
- **Stats overlay:** FPS, frame time, triangles, meshes, camera position,
  distance to the centre, the height above the ground (on a planet or moon)
  or the level's size (above the planet), and the level.

## Select and show

- **Select:** click an object in the 3D view, or its line in the Nodes list.
  - Both always show the same selection, and the selected object wears an
    orange outline.
  - A click on empty space clears the selection.
  - A drag orbits and doesn't select.
- **F, or Show selected, on a moon:** the moon becomes the body on show.
  - It is drawn at the centre with the game's planet surface code at its
    own size and metre.
  - The list starts from it, and the readouts are its own (the Moon reads
    1,738 km).
  - Parent, or Page Up, returns to the planet.
- **F above the planet:** opens the selected object, the same as Child.

## Type gallery

`engine-lab --screens` ends with one picture per planet type and per
moon type (`gallery-planet-<slug>.png`, `gallery-moon-<slug>.png`),
each framed on its pinned seed (#685, ADR 0031). The gallery shots draw
the Generated look, so relief, colours, bands and moon shapes read;
every other shot draws Normal. The pins live in
`universe_core::world::gallery` (`PLANET_GALLERY`, `MOON_GALLERY`); if a
generator change moves a pin, the gallery tests fail and the ignored
`pin_scan` re-searches them.

## Levels

- **Parent** (button, Page Up) shows the cell one level up, as the game
  closes a cell. Examples:
  - from a planet (L10) to its star system (L9);
  - all the way up to L1.
- **Child** (button, Page Down, or F) opens the selected object one level
  down, as the game opens it, for example a planet in its star system.
- Each level is drawn by the game's own drawing.
- Home frames the whole cell above the planet, and the planet with its
  moons on a planet.
- Wireframe, Normals and LOD depth view belong to a planet's (or moon's)
  surface, so they are greyed out at L1-L9.
- Nothing opens below a planet: L11-L14 are height shells around it (#632).

## Cameras and keys

| Input | Orbit (default) | Free-fly (G) |
|---|---|---|
| Left-click | Select the object under the pointer (empty space clears) | |
| Left-drag | Turn around the target | — |
| Middle-drag | Pan | — |
| Wheel | Zoom | — |
| Right-drag | — | Look |
| W A S D | — | Fly; the speed follows the height above the ground |
| Shift+1-9 | — | Speed step |
| F | Show the selected object | Show the selected object |
| G | Switch to Free-fly | Back to Orbit |
| Page Up / Page Down | Parent / Child level | Parent / Child level |
| Numpad 7 / Ctrl+Numpad 7 | Top / Bottom | |
| Numpad 1 / Ctrl+Numpad 1 | Front / Back | |
| Numpad 3 / Ctrl+Numpad 3 | Right / Left | |
| Numpad 0 | Perspective | |
| Home | Home view (the planet and its moons, or the whole cell) | |
| F12 | Screenshot into `screenshots/engine-lab-YYYY-MM-DD-HHMMSS.png` | |

## Shaders

The lab shows the game's shaders on the body on show: the planet, or a
moon shown with F (#665). Today there is one, **Planet squares** (#626,
ADR 0027).
- **Select:** click "Planet squares" under "Shaders" in the Nodes list, or
  tick it in the Shaders menu.
- **On/off:** on draws the game's Squares look, and off draws its Normal
  look. The lab starts with it off, as the game does.
- **Settings** change the picture at once:
  - Square size: 1 m to 10 km, default 100 m.
  - Colour A and Colour B: default black and white.
  - Fade: sharp 2 to 64 px (default 8) and even 1 to 16 px (default 2).
    The even size always stays below the sharp one.
- **Reset** gives back the game's values. The values are lab-only, are
  never saved, and start from the game's values on every run.
- **Copy values** puts the settings on the clipboard as Rust constants
  named as in `universe_core::squares` (`SQUARE_M`, `SHARP_PX`,
  `GRAY_PX`, `COLOUR_A`, `COLOUR_B`). The status bar says "Values copied".
  - It uses the platform's own command: `clip` on Windows, `pbcopy` on
    macOS, and `wl-copy` or `xclip` on Linux. No clipboard crate is used,
    because `arboard`'s Windows backend is not on the `deny.toml` licence
    list.
- **Debug view** (tools panel and Shaders menu) replaces the surface
  colour, with the squares on or off:
  - Off: the normal picture.
  - Position: red, green and blue bands repeating every square along the
    planet's x, y and z.
  - Normal: the planet-frame direction the surface faces, as red, green
    and blue. The "Normals" toggle, which draws short lines, is separate.
  - Distance: bright near the camera to dark far away, over a range you
    can set (default 0 m to 10 km).
- At L1-L9 there is no surface, so all of it is greyed out.

The lab writes the game's `SettingsStore.planet_squares` (its in-memory
copy) and `universe_render::SquaresTuning`. The game never changes
`SquaresTuning`, so it always draws 100 m black-and-white squares and no
debug view.

## Shader code

The Shaders menu and the Planet squares panel have an **Edit code** entry
(#666). It opens a separate **Shader code** window over the view with the
game's Planet squares code as text.
- **Save** applies the edited text in the lab only: the body on show
  redraws at once. The game's file on disk never changes.
- The lab checks the text first with the same WGSL parser Bevy uses. When
  the text has an error, the picture stays as it was and the window shows
  the message; fix the text and press Save again.
- **Imports** (`#import` lines) stay as in the game's shader: Bevy
  resolves them and the lab cannot check them, so changed imports are
  refused with a message.
- **Reset** gives back the game's code. Edits are lab-only, are never
  saved, and start from the game's code with the window closed on every
  run.
- **Copy code** puts the exact edited text on the clipboard as plain text,
  to paste into the game's code by hand. The status bar says "Code copied".
- At L1-L9 there is no surface, so Save is greyed out.
- When Bevy itself rejects applied text (for example a wrong bind group),
  the lab puts the game's code back, shows Bevy's message, and keeps
  running.

## Shader library

The Shaders menu and the Shaders line have a **New shader** entry (#667).
It adds a shader with the next free name ("Shader 1", "Shader 2", and so
on) and selects it at once.

- **One at a time:** selecting a library entry shows it on the body on
  show at once. Each shader remembers its own on/off, settings, code and
  error while the lab runs.
- **Blank start:** a new shader draws the Normal look until you edit it.
  Its code window works exactly like the Planet squares one: Save applies
  the edited text in the lab only, an error keeps the last good picture
  and shows its message, and Reset gives back the blank start.
- **Same controls:** every shader has the same on/off, Square size,
  colours, fade, Reset, Copy values, debug views and code window, driving
  the same values. Moving the sliders on a blank shader does nothing
  visible until its code uses those values.
- **Rename and Delete** sit in the shader's tools panel. Names stay
  unique (the lab adds a number when needed). Deleting the shown shader
  falls back to the Normal look on Planet squares. Planet squares itself
  cannot be renamed or deleted.
- **Lab-only:** created shaders, their code and their settings are never
  saved. A restart starts again with only Planet squares (off, game
  values, window closed).
- **To the game by hand:** Copy code and Copy values put the shader's
  text and settings on the clipboard, to paste into the game's code. The
  lab never writes game files and the game never changes by itself.
- At L1-L9 there is no surface, so New shader, the shader controls and
  Save are greyed out.

## The planet

The lab opens on the game's home star and its home planet: Earth with its
Moon on the default seed. The home star is the first eight markers of the
Spacebar journey.
- A new seed rebuilds the universe on that seed's home star.
- The path field (`7/2/7/1/2/0/0/0`, L1 to L9) jumps to another star.
- The same seed and path always give the same planet.

The drawing is the game's own, through `universe_render::PlanetViewPlugin`:
- the open cell's markers at every level;
- the icosphere surface and its LOD (#593, #598);
- the squares look (#626);
- the moons and the air rim.

The lab sets the camera pose, the `SurfaceDebug` views and the
`Universe` on show, and reads `SurfaceStats`.

The overlay reads distances in the game's planet metre (#648, ADR 0030):
the planet's ground sits at its real radius. On Earth the distance to the
centre therefore reads:
- 6,371 km at a corner of a face;
- about 5,950 to 6,015 km in the middle of one, because the faces are flat.

This matches the size readout. Above the planet, distances read in the
level's ladder metre.
