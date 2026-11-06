# #1162 T7: before vs after, per scenario

Medians from each run's raw `route/frames.txt`, written by `fixtures/compare.py`; change = after/before - 1 (ms). Before: `before/` (exe ffc9fc87…, the f943143 build). After: `after/` (`after/exe.sha256`, commit in `after/commit.txt`).
Limit (criterion 9): fixed-quality interval median within +5%, no new stalls over 100 ms.

## water-fixed

| scenario | interval before | after | change | render before | after | cpu before | after | >100 ms before/after | ready ms before/after |
|---|---|---|---|---|---|---|---|---|---|
| home-coast-ground | 36.98 | 37.94 | +2.6% | 8.00 | 7.88 | 21.06 | 20.73 | 0/0 | 103.0/82.0 |
| home-coast-low | 47.31 | 48.93 | +3.4% | 8.00 | 8.05 | 15.68 | 16.02 | 0/0 | 442.0/430.0 |
| home-orbit | 49.20 | 51.22 | +4.1% | 7.66 | 7.68 | 32.92 | 33.44 | 5/4 | 145.0/119.0 |
| home-shallow | 34.39 | 39.61 | +15.2% | 7.40 | 7.37 | 11.52 | 11.67 | 0/0 | 2866.0/2544.0 |
| ice-coast-ground | 37.48 | 41.62 | +11.0% | 8.05 | 8.15 | 22.35 | 21.69 | 1/1 | 2205.0/2386.0 |
| ice-coast-low | 36.23 | 40.62 | +12.1% | 7.76 | 7.80 | 11.25 | 11.82 | 0/0 | 110.0/117.0 |
| desert-coast-ground | 44.00 | 46.76 | +6.3% | 8.16 | 7.81 | 20.81 | 19.61 | 1/1 | 2566.0/2577.0 |
| desert-coast-low | 42.54 | 44.89 | +5.5% | 7.80 | 8.12 | 11.13 | 11.89 | 0/0 | 123.0/105.0 |
| home-again | 36.11 | 38.25 | +5.9% | 7.45 | 7.46 | 11.07 | 11.59 | 1/1 | 2052.0/2614.0 |
| ice-again | 30.42 | 33.87 | +11.3% | 7.77 | 7.34 | 12.97 | 11.68 | 1/1 | 2268.0/2282.0 |
| desert-again | 34.25 | 36.10 | +5.4% | 7.74 | 7.41 | 11.65 | 11.41 | 1/1 | 1167.0/1776.0 |
| **all** | 36.32 | 39.07 | +7.6% | 7.85 | 7.71 | 13.49 | 13.80 | 10/9 | warm 2428.0/2871.0 |

## water-terrain-low

| scenario | interval before | after | change | render before | after | cpu before | after | >100 ms before/after | ready ms before/after |
|---|---|---|---|---|---|---|---|---|---|
| home-coast-ground | 35.08 | 35.73 | +1.9% | 8.12 | 7.92 | 20.84 | 20.83 | 0/0 | 96.0/110.0 |
| home-coast-low | 41.66 | 42.49 | +2.0% | 7.98 | 7.94 | 14.55 | 14.82 | 0/0 | 486.0/496.0 |
| home-orbit | 45.28 | 46.07 | +1.7% | 7.74 | 7.72 | 26.55 | 26.59 | 3/0 | 109.0/105.0 |
| home-shallow | 33.34 | 38.39 | +15.1% | 7.85 | 7.45 | 11.85 | 11.65 | 0/0 | 2531.0/2236.0 |
| ice-coast-ground | 34.26 | 37.36 | +9.1% | 8.22 | 8.18 | 22.46 | 21.23 | 1/1 | 2062.0/2115.0 |
| ice-coast-low | 32.26 | 35.59 | +10.3% | 7.81 | 7.78 | 10.63 | 11.19 | 0/0 | 104.0/96.0 |
| desert-coast-ground | 39.78 | 40.95 | +2.9% | 8.18 | 8.15 | 20.54 | 20.57 | 1/1 | 2192.0/2655.0 |
| desert-coast-low | 40.02 | 41.11 | +2.7% | 7.92 | 7.76 | 10.81 | 10.84 | 0/0 | 93.0/100.0 |
| home-again | 35.31 | 37.01 | +4.8% | 7.56 | 7.48 | 10.93 | 11.23 | 1/0 | 1952.0/2611.0 |
| ice-again | 29.30 | 32.65 | +11.4% | 7.73 | 7.36 | 12.60 | 11.40 | 1/1 | 1950.0/2072.0 |
| desert-again | 33.53 | 34.64 | +3.3% | 7.90 | 7.46 | 11.73 | 11.25 | 1/1 | 800.0/1317.0 |
| **all** | 34.73 | 36.49 | +5.1% | 7.94 | 7.74 | 13.03 | 12.73 | 8/4 | warm 2241.0/2880.0 |

## water-terrain-high

| scenario | interval before | after | change | render before | after | cpu before | after | >100 ms before/after | ready ms before/after |
|---|---|---|---|---|---|---|---|---|---|
| home-coast-ground | 45.59 | 47.17 | +3.5% | 8.92 | 8.70 | 38.85 | 37.88 | 0/0 | 145.0/155.0 |
| home-coast-low | 62.22 | 64.61 | +3.8% | 8.39 | 8.40 | 23.12 | 22.88 | 0/1 | 1038.0/900.0 |
| home-orbit | 59.99 | 62.56 | +4.3% | 7.79 | 7.89 | 41.38 | 42.79 | 34/26 | 144.0/135.0 |
| home-shallow | 40.72 | 47.98 | +17.9% | 7.47 | 7.43 | 14.42 | 14.43 | 1/0 | 5386.0/5404.0 |
| ice-coast-ground | 51.04 | 56.76 | +11.2% | 8.25 | 7.98 | 32.68 | 30.11 | 1/1 | 3541.0/3431.0 |
| ice-coast-low | 50.47 | 58.55 | +16.0% | 7.90 | 8.87 | 14.15 | 15.82 | 0/0 | 123.0/156.0 |
| desert-coast-ground | 50.75 | 59.73 | +17.7% | 7.82 | 7.93 | 31.41 | 26.67 | 1/1 | 14916.0/5480.0 |
| desert-coast-low | 60.05 | 63.63 | +6.0% | 9.81 | 10.36 | 15.07 | 16.03 | 0/0 | 123.0/169.0 |
| home-again | 43.39 | 46.91 | +8.1% | 7.37 | 7.49 | 14.34 | 14.59 | 1/1 | 3485.0/4496.0 |
| ice-again | 35.66 | 41.61 | +16.7% | 7.60 | 7.32 | 14.88 | 14.28 | 1/1 | 3464.0/3377.0 |
| desert-again | 39.59 | 41.59 | +5.1% | 7.38 | 7.41 | 18.52 | 13.57 | 1/1 | 14236.0/5675.0 |
| **all** | 44.01 | 47.99 | +9.0% | 7.82 | 7.85 | 19.70 | 19.10 | 40/32 | warm 4289.0/4398.0 |

## water-auto

| scenario | interval before | after | change | render before | after | cpu before | after | >100 ms before/after | ready ms before/after |
|---|---|---|---|---|---|---|---|---|---|
| home-coast-ground | 36.12 | 38.05 | +5.3% | 5.42 | 5.50 | 10.34 | 10.72 | 0/0 | 117.0/107.0 |
| home-coast-low | 38.96 | 38.44 | -1.3% | 6.04 | 6.04 | 9.02 | 8.90 | 0/0 | 427.0/300.0 |
| home-orbit | 46.44 | 45.98 | -1.0% | 5.41 | 5.76 | 14.39 | 15.92 | 0/0 | 98.0/94.0 |
| home-shallow | 26.73 | 31.07 | +16.2% | 7.85 | 7.49 | 12.21 | 11.13 | 0/0 | 2174.0/2243.0 |
| ice-coast-ground | 27.85 | 31.35 | +12.6% | 8.63 | 8.39 | 22.76 | 22.41 | 1/1 | 1979.0/2038.0 |
| ice-coast-low | 26.60 | 29.83 | +12.1% | 8.02 | 8.02 | 11.33 | 10.98 | 0/0 | 55.0/86.0 |
| desert-coast-ground | 32.43 | 34.11 | +5.2% | 8.32 | 8.34 | 21.61 | 21.11 | 1/1 | 1994.0/2327.0 |
| desert-coast-low | 32.95 | 34.11 | +3.5% | 7.78 | 7.65 | 10.07 | 10.38 | 0/0 | 85.0/83.0 |
| home-again | 32.35 | 32.98 | +1.9% | 7.59 | 7.64 | 10.59 | 11.05 | 1/0 | 1894.0/2610.0 |
| ice-again | 27.26 | 27.37 | +0.4% | 8.02 | 7.54 | 12.69 | 11.77 | 1/1 | 1987.0/2092.0 |
| desert-again | 30.54 | 36.80 | +20.5% | 7.74 | 6.81 | 10.77 | 8.84 | 1/1 | 826.0/1064.0 |
| **all** | 30.66 | 32.90 | +7.3% | 7.87 | 7.65 | 11.93 | 11.38 | 5/4 | warm 2810.0/2881.0 |

## Auto settles (water-auto, end of each leg)

Settings shown as terrain / markers / atmosphere / render_scale.

| leg | before | after | same |
|---|---|---|---|
| 0 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 1 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 3 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 4 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 5 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 6 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 7 | low/low/normal/0.8 | low/low/normal/0.9 | NO |
| 8 | low/low/normal/0.8 | low/low/normal/0.8 | yes |
| 9 | low/low/normal/0.9 | low/low/normal/0.9 | yes |
| 10 | low/low/normal/1 | low/low/normal/0.9 | NO |
| 11 | low/low/normal/0.9 | low/low/normal/0.8 | NO |
| 12 | low/low/normal/0.9 | low/low/normal/0.8 | NO |
| 13 | low/low/normal/0.9 | low/low/normal/0.8 | NO |

## Working set during each visit (water-fixed, MB median while there)

| visit | before | after | change |
|---|---|---|---|
| home | 890 | 895 | +0.5% |
| ice | 913 | 916 | +0.4% |
| desert | 913 | 917 | +0.5% |
| home again | 914 | 919 | +0.5% |
| ice again | 915 | 919 | +0.5% |
| desert again | 917 | 920 | +0.4% |
