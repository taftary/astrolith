# Planet lab

A development program for planet and moon generation (#640, ADR 0029). It
shows one planet of the game's universe with its moons (the L10-L14 world),
drawn with the game's own planet code. It is never part of the game and is
never shipped to players. It runs on Windows, Linux and macOS desktops only.

## Run

```
cargo run -p planet-lab
cargo run -p planet-lab -- --screens <folder>
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
  - Camera: Orbit / Free-fly (F), Home, and the seven preset views.
  - Help: About.
- **Nodes** (left): Planet, one line per moon, Camera, Light, Grid, Axes.
  Click a node to select it.
- **Tools and options** (right):
  - the seed and path fields with Regenerate;
  - Previous, Next and a slider over the star's planets ("Planet i of N");
  - the selected node's values in real metres;
  - the Planet node's debug toggles, with the splits per LOD depth when
    the LOD depth view is on.
  - A wrong seed or path shows a red message and keeps the planet.
- **Status bar** (bottom): the seed, the path, "Planet i of N" and the last
  saved screenshot.
- **Stats overlay:** FPS, frame time, triangles, meshes, camera position,
  distance to the planet centre, height above the ground, and the shell
  level L10-L14 (#632).

## Cameras and keys

| Input | Orbit (default) | Free-fly (F) |
|---|---|---|
| Left-drag | Turn around the target | — |
| Middle-drag | Pan | — |
| Wheel | Zoom | — |
| Right-drag | — | Look |
| W A S D | — | Fly; the speed follows the height above the ground |
| Shift+1-9 | — | Speed step |
| Numpad 7 / Ctrl+Numpad 7 | Top / Bottom | |
| Numpad 1 / Ctrl+Numpad 1 | Front / Back | |
| Numpad 3 / Ctrl+Numpad 3 | Right / Left | |
| Numpad 0 | Perspective | |
| Home | Home view (the planet and its moons) | |
| F12 | Screenshot into `screenshots/planet-lab-YYYY-MM-DD-HHMMSS.png` | |

## The planet

The lab opens on the game's home star (the first eight markers of the
Spacebar journey) and its home planet: Earth with its Moon on the default
seed. A new seed rebuilds the universe on that seed's home star. The path
field (`7/2/7/1/2/0/0/0`, L1 to L9) jumps to another star. The same seed and
path always give the same planet.

The drawing is the game's own, through `universe_render::PlanetViewPlugin`:
the icosphere surface and its LOD (#593, #598), the squares look (#626), the
moons, the air rim and the open cell's markers. The lab only sets the camera
pose and the `SurfaceDebug` views, and reads `SurfaceStats`.

The overlay reads distances in the game's planet metre (#648, ADR 0030):
the planet's ground sits at its real radius, so on Earth the distance to the
centre reads 6,371 km at a corner of a face and about 5,950 to 6,015 km in
the middle of one (the faces are flat), matching the size readout.
