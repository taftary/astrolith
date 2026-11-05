# #1161 before: frame and memory tables

Copied by `fixtures/tables.py` from each route's `summary.json`, `resources.json` and `memory.jsonl` (ms; MB = 10^6 bytes).

## surfaces-fixed

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 40.10 | 51.59 | 6.96 | 10.31 | 2 | 2168.0 |
| home-ground | 39.82 | 48.95 | 5.05 | 11.38 | 0 | 113.0 |
| home-low | 42.60 | 51.26 | 6.64 | 11.04 | 0 | 258.0 |
| home-orbit | 46.38 | 49.41 | 8.63 | 10.72 | 0 | 140.0 |
| mercury-ground | 39.20 | 48.61 | 5.19 | 11.69 | 0 | 208.0 |
| mercury-low | 42.28 | 50.72 | 7.07 | 11.52 | 0 | 97.0 |
| mercury-orbit | 45.88 | 48.95 | 8.63 | 10.68 | 0 | 110.0 |
| desert-ground | 39.75 | 49.14 | 5.43 | 11.91 | 0 | 167.0 |
| desert-low | 42.46 | 50.93 | 7.06 | 11.46 | 0 | 108.0 |
| desert-orbit | 46.22 | 49.34 | 8.69 | 10.80 | 0 | 126.0 |
| iron-ground | 39.76 | 48.85 | 5.34 | 11.74 | 0 | 195.0 |
| iron-low | 42.48 | 50.85 | 7.02 | 11.45 | 0 | 102.0 |
| iron-orbit | 46.20 | 49.32 | 8.70 | 10.94 | 0 | 130.0 |
| ice-ground | 39.81 | 49.22 | 5.33 | 11.68 | 0 | 195.0 |
| ice-low | 39.84 | 49.93 | 6.19 | 10.42 | 0 | 94.0 |
| ice-orbit | 39.82 | 51.48 | 7.58 | 9.83 | 0 | 95.0 |
| lava-ground | 39.89 | 48.98 | 5.21 | 11.38 | 0 | 133.0 |
| lava-low | 42.17 | 49.96 | 6.96 | 11.53 | 0 | 117.0 |
| lava-orbit | 45.32 | 48.37 | 8.59 | 10.63 | 0 | 122.0 |
| **all** | 41.71 | 49.57 | 6.55 | 11.24 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 15.9 last / 17.4 peak. Working set MB: peak 908, last 40 (451 samples).

## surfaces-terrain-low

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 40.14 | 51.43 | 7.22 | 10.40 | 2 | 2003.0 |
| home-ground | 39.86 | 48.87 | 5.26 | 11.28 | 0 | 100.0 |
| home-low | 40.64 | 51.38 | 6.36 | 10.24 | 0 | 316.0 |
| home-orbit | 42.17 | 44.81 | 8.13 | 9.99 | 0 | 114.0 |
| mercury-ground | 39.70 | 48.83 | 5.44 | 11.59 | 0 | 159.0 |
| mercury-low | 40.73 | 50.62 | 6.72 | 10.77 | 0 | 107.0 |
| mercury-orbit | 41.75 | 44.63 | 8.27 | 10.13 | 0 | 112.0 |
| desert-ground | 39.78 | 48.90 | 5.34 | 11.34 | 0 | 170.0 |
| desert-low | 40.77 | 50.33 | 6.57 | 10.41 | 0 | 95.0 |
| desert-orbit | 41.77 | 44.68 | 8.36 | 10.24 | 0 | 125.0 |
| iron-ground | 39.73 | 49.19 | 5.42 | 11.64 | 0 | 141.0 |
| iron-low | 40.69 | 50.77 | 6.66 | 10.43 | 0 | 82.0 |
| iron-orbit | 41.79 | 44.89 | 8.40 | 10.32 | 0 | 106.0 |
| ice-ground | 39.74 | 49.05 | 5.32 | 11.36 | 0 | 181.0 |
| ice-low | 39.84 | 50.23 | 6.39 | 9.94 | 0 | 86.0 |
| ice-orbit | 39.80 | 50.62 | 7.76 | 9.42 | 0 | 81.0 |
| lava-ground | 39.87 | 49.14 | 5.21 | 11.18 | 0 | 153.0 |
| lava-low | 40.29 | 50.94 | 6.57 | 10.58 | 0 | 125.0 |
| lava-orbit | 41.14 | 43.89 | 8.21 | 10.11 | 0 | 116.0 |
| **all** | 40.43 | 49.27 | 6.47 | 10.73 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 14.3 last / 15.9 peak. Working set MB: peak 899, last 429 (451 samples).

## surfaces-terrain-high

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 40.07 | 50.21 | 6.64 | 12.71 | 2 | 3560.0 |
| home-ground | 39.80 | 46.86 | 5.00 | 15.44 | 0 | 94.0 |
| home-low | 50.97 | 58.83 | 7.88 | 20.05 | 0 | 1002.0 |
| home-orbit | 55.58 | 58.92 | 8.97 | 11.64 | 0 | 119.0 |
| mercury-ground | 33.21 | 41.63 | 8.57 | 30.53 | 0 | 216.0 |
| mercury-low | 50.20 | 58.67 | 8.48 | 20.23 | 0 | 75.0 |
| mercury-orbit | 55.14 | 58.57 | 8.85 | 11.43 | 0 | 146.0 |
| desert-ground | 33.12 | 41.45 | 8.53 | 30.43 | 0 | 243.0 |
| desert-low | 50.36 | 59.00 | 8.48 | 20.17 | 0 | 90.0 |
| desert-orbit | 55.65 | 59.07 | 9.18 | 11.81 | 0 | 129.0 |
| iron-ground | 33.24 | 41.82 | 8.61 | 30.72 | 0 | 244.0 |
| iron-low | 50.36 | 58.77 | 8.43 | 20.18 | 0 | 97.0 |
| iron-orbit | 55.85 | 59.21 | 9.03 | 11.64 | 0 | 138.0 |
| ice-ground | 30.28 | 38.54 | 8.12 | 28.01 | 0 | 228.0 |
| ice-low | 41.62 | 46.69 | 8.08 | 19.94 | 0 | 84.0 |
| ice-orbit | 44.26 | 47.21 | 8.41 | 11.17 | 0 | 90.0 |
| lava-ground | 32.74 | 40.93 | 8.79 | 30.31 | 0 | 188.0 |
| lava-low | 48.65 | 57.36 | 8.40 | 20.12 | 0 | 94.0 |
| lava-orbit | 54.14 | 57.79 | 9.00 | 11.79 | 0 | 140.0 |
| **all** | 41.54 | 57.81 | 8.28 | 22.35 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 32.4 last / 34.0 peak. Working set MB: peak 919, last 190 (451 samples).

## surfaces-auto

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 40.13 | 51.87 | 7.72 | 10.53 | 2 | 2526.0 |
| home-ground | 39.83 | 48.96 | 5.63 | 11.19 | 0 | 97.0 |
| home-low | 39.86 | 51.11 | 6.56 | 9.88 | 0 | 276.0 |
| home-orbit | 39.82 | 50.51 | 8.30 | 9.50 | 0 | 124.0 |
| mercury-ground | 39.83 | 48.53 | 5.41 | 10.66 | 0 | 123.0 |
| mercury-low | 39.81 | 51.07 | 6.93 | 10.08 | 0 | 119.0 |
| mercury-orbit | 40.45 | 51.56 | 7.86 | 9.65 | 0 | 104.0 |
| desert-ground | 39.73 | 48.99 | 5.39 | 11.52 | 0 | 146.0 |
| desert-low | 40.77 | 51.19 | 6.53 | 10.27 | 0 | 82.0 |
| desert-orbit | 41.74 | 44.62 | 8.33 | 10.21 | 0 | 105.0 |
| iron-ground | 39.75 | 49.04 | 5.42 | 11.64 | 0 | 162.0 |
| iron-low | 40.77 | 50.30 | 6.72 | 10.72 | 0 | 105.0 |
| iron-orbit | 44.32 | 49.01 | 8.56 | 10.53 | 0 | 126.0 |
| ice-ground | 39.78 | 49.01 | 5.28 | 11.60 | 0 | 183.0 |
| ice-low | 39.81 | 49.94 | 6.11 | 10.08 | 0 | 104.0 |
| ice-orbit | 39.82 | 51.34 | 7.68 | 9.53 | 0 | 94.0 |
| lava-ground | 39.87 | 48.95 | 5.20 | 11.25 | 0 | 156.0 |
| lava-low | 42.14 | 50.10 | 6.92 | 11.40 | 0 | 103.0 |
| lava-orbit | 46.60 | 51.45 | 9.29 | 11.04 | 0 | 131.0 |
| **all** | 40.60 | 49.79 | 6.55 | 10.70 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/14, ground materials 1/32/32, terrain MB 16.0 last / 17.6 peak. Working set MB: peak 906, last 417 (451 samples).
