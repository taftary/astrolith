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
  - Help: About.
- **Nodes** (left):
  - On a planet: Planet, one line per moon, then Camera, Light, Grid and Axes.
  - With a moon on show: that moon, then the tools.
  - Above the planet (L1-L9): the level's own objects by their game names,
    then the tools.
  - Click a line to select it.
- **Tools and options** (right):
  - the seed and path fields with Regenerate;
  - Parent, Child and Show selected (F), with the level and path on show;
  - Previous, Next and a slider over the star's planets ("Planet i of N"),
    on a planet only;
  - the selected node's values in real metres;
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
