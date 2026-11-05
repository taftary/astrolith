# #1161 after: frame and memory tables

Copied by `fixtures/tables.py` from each route's `summary.json`, `resources.json` and `memory.jsonl` (ms; MB = 10^6 bytes).

## surfaces-fixed

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 35.38 | 39.38 | 7.48 | 12.28 | 2 | 2770.0 |
| home-ground | 35.79 | 38.18 | 8.47 | 23.83 | 0 | 74.0 |
| home-low | 45.28 | 54.61 | 8.29 | 16.71 | 0 | 348.0 |
| home-orbit | 50.67 | 53.75 | 8.54 | 10.96 | 0 | 141.0 |
| mercury-ground | 35.73 | 37.12 | 8.45 | 23.16 | 0 | 178.0 |
| mercury-low | 44.92 | 54.27 | 8.33 | 15.99 | 0 | 85.0 |
| mercury-orbit | 50.29 | 53.70 | 8.57 | 11.02 | 0 | 147.0 |
| desert-ground | 35.84 | 37.40 | 8.46 | 23.33 | 0 | 176.0 |
| desert-low | 45.22 | 54.79 | 8.44 | 16.35 | 0 | 75.0 |
| desert-orbit | 50.80 | 54.09 | 8.54 | 11.11 | 0 | 142.0 |
| iron-ground | 35.69 | 37.33 | 8.48 | 23.30 | 0 | 172.0 |
| iron-low | 44.95 | 54.64 | 8.32 | 16.14 | 0 | 89.0 |
| iron-orbit | 50.90 | 54.05 | 8.32 | 10.74 | 0 | 119.0 |
| ice-ground | 32.54 | 34.16 | 8.44 | 24.00 | 0 | 189.0 |
| ice-low | 38.26 | 43.39 | 8.14 | 16.24 | 0 | 90.0 |
| ice-orbit | 41.20 | 43.22 | 7.65 | 9.74 | 0 | 119.0 |
| lava-ground | 36.21 | 39.36 | 9.00 | 25.20 | 0 | 164.0 |
| lava-low | 44.76 | 53.61 | 8.60 | 17.29 | 0 | 72.0 |
| lava-orbit | 49.83 | 52.91 | 8.41 | 10.96 | 0 | 109.0 |
| **all** | 40.71 | 52.97 | 8.44 | 18.72 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 15.9 last / 17.4 peak. Working set MB: peak 904, last 203 (451 samples).

## surfaces-terrain-low

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 33.61 | 35.68 | 7.82 | 11.62 | 4 | 2802.0 |
| home-ground | 34.56 | 36.23 | 8.41 | 23.11 | 0 | 82.0 |
| home-low | 40.14 | 49.72 | 8.15 | 15.34 | 0 | 351.0 |
| home-orbit | 45.57 | 48.76 | 7.80 | 9.92 | 0 | 146.0 |
| mercury-ground | 34.37 | 35.87 | 8.40 | 22.54 | 0 | 164.0 |
| mercury-low | 39.87 | 49.33 | 8.03 | 14.79 | 0 | 81.0 |
| mercury-orbit | 45.50 | 48.39 | 7.82 | 10.80 | 0 | 119.0 |
| desert-ground | 34.40 | 36.00 | 8.47 | 22.79 | 0 | 190.0 |
| desert-low | 39.95 | 49.43 | 8.09 | 15.01 | 0 | 97.0 |
| desert-orbit | 45.63 | 48.87 | 7.83 | 10.23 | 0 | 136.0 |
| iron-ground | 34.35 | 35.83 | 8.44 | 22.54 | 0 | 186.0 |
| iron-low | 40.02 | 49.67 | 8.17 | 15.25 | 0 | 83.0 |
| iron-orbit | 45.70 | 48.89 | 7.82 | 9.77 | 0 | 113.0 |
| ice-ground | 31.46 | 32.87 | 8.33 | 23.39 | 0 | 190.0 |
| ice-low | 34.38 | 40.85 | 8.06 | 15.40 | 0 | 81.0 |
| ice-orbit | 37.08 | 38.57 | 7.39 | 9.66 | 0 | 102.0 |
| lava-ground | 33.90 | 35.82 | 8.51 | 22.85 | 0 | 137.0 |
| lava-low | 38.97 | 48.48 | 8.04 | 14.87 | 0 | 79.0 |
| lava-orbit | 45.32 | 47.80 | 7.69 | 9.87 | 0 | 135.0 |
| **all** | 37.65 | 47.90 | 8.16 | 17.58 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 14.3 last / 15.9 peak. Working set MB: peak 901, last 75 (451 samples).

## surfaces-terrain-high

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 40.77 | 44.96 | 7.74 | 17.13 | 2 | 3956.0 |
| home-ground | 41.36 | 46.96 | 9.32 | 37.54 | 0 | 84.0 |
| home-low | 57.86 | 66.08 | 8.70 | 23.22 | 0 | 1011.0 |
| home-orbit | 61.45 | 64.80 | 9.22 | 12.34 | 0 | 179.0 |
| mercury-ground | 41.12 | 43.11 | 9.25 | 35.33 | 1 | 255.0 |
| mercury-low | 57.47 | 66.15 | 8.84 | 21.77 | 0 | 128.0 |
| mercury-orbit | 60.87 | 64.25 | 9.16 | 12.38 | 0 | 161.0 |
| desert-ground | 41.22 | 43.15 | 9.25 | 35.36 | 0 | 216.0 |
| desert-low | 57.22 | 65.64 | 8.79 | 21.28 | 0 | 93.0 |
| desert-orbit | 61.67 | 65.02 | 9.19 | 12.36 | 0 | 162.0 |
| iron-ground | 41.04 | 43.31 | 9.29 | 35.79 | 0 | 235.0 |
| iron-low | 57.24 | 65.45 | 8.85 | 21.54 | 0 | 104.0 |
| iron-orbit | 61.66 | 65.23 | 9.24 | 12.43 | 0 | 186.0 |
| ice-ground | 38.19 | 44.64 | 9.45 | 35.39 | 0 | 287.0 |
| ice-low | 48.21 | 54.88 | 8.50 | 21.24 | 0 | 92.0 |
| ice-orbit | 50.01 | 53.20 | 8.45 | 11.33 | 0 | 125.0 |
| lava-ground | 40.55 | 43.22 | 9.28 | 35.55 | 0 | 215.0 |
| lava-low | 55.37 | 63.27 | 8.75 | 21.29 | 0 | 103.0 |
| lava-orbit | 60.02 | 63.55 | 9.23 | 12.28 | 0 | 141.0 |
| **all** | 48.59 | 64.01 | 9.10 | 26.71 | 1 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/13, ground materials 1/32/32, terrain MB 32.4 last / 34.0 peak. Working set MB: peak 920, last 44 (451 samples).

## surfaces-auto

| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |
|---|---|---|---|---|---|---|
| warmup | 28.15 | 30.38 | 7.99 | 12.95 | 3 | 2888.0 |
| home-ground | 28.80 | 30.56 | 9.17 | 24.28 | 0 | 61.0 |
| home-low | 32.12 | 37.24 | 8.29 | 15.52 | 0 | 331.0 |
| home-orbit | 37.88 | 41.55 | 7.52 | 9.19 | 0 | 97.0 |
| mercury-ground | 31.77 | 34.06 | 8.94 | 23.75 | 0 | 179.0 |
| mercury-low | 36.41 | 43.44 | 8.41 | 14.94 | 0 | 85.0 |
| mercury-orbit | 40.73 | 45.48 | 7.68 | 9.28 | 0 | 92.0 |
| desert-ground | 31.62 | 33.59 | 8.91 | 23.75 | 0 | 161.0 |
| desert-low | 36.57 | 44.22 | 8.38 | 14.87 | 0 | 84.0 |
| desert-orbit | 40.66 | 42.82 | 7.83 | 9.64 | 0 | 111.0 |
| iron-ground | 32.08 | 33.38 | 8.85 | 23.92 | 0 | 168.0 |
| iron-low | 36.44 | 43.40 | 8.36 | 14.99 | 0 | 92.0 |
| iron-orbit | 44.58 | 48.06 | 7.84 | 10.14 | 0 | 115.0 |
| ice-ground | 31.05 | 32.90 | 8.61 | 24.08 | 0 | 161.0 |
| ice-low | 34.19 | 40.15 | 8.05 | 15.42 | 0 | 92.0 |
| ice-orbit | 39.74 | 42.47 | 7.72 | 9.95 | 0 | 104.0 |
| lava-ground | 34.77 | 36.46 | 8.59 | 23.52 | 0 | 164.0 |
| lava-low | 43.53 | 53.06 | 8.39 | 16.06 | 0 | 93.0 |
| lava-orbit | 49.66 | 52.92 | 8.56 | 10.95 | 0 | 106.0 |
| **all** | 35.36 | 46.85 | 8.45 | 18.10 | 0 | |

Resources (first / last / peak): meshes 8/8/8, images 7/13/14, ground materials 1/32/32, terrain MB 16.0 last / 17.4 peak. Working set MB: peak 913, last 859 (450 samples).
