# #1160 T1 baseline: frame numbers per scenario

Source: `<run>/route/frames.txt`, columns time, scenario, interval, cpu work, main, render, wait (ms). The median and p95 are computed from the raw rows. `over_100ms` comes from `summary.json`.

## light-fixed

| scenario | frames | interval median | interval p95 | cpu work median | render median | over 100 ms |
|---|---|---|---|---|---|---|
| warmup | 748 | 36.86 | 51.32 | 8.91 | 6.09 | 2 |
| day | 1505 | 38.76 | 49.00 | 10.87 | 4.82 | 0 |
| dusk | 1505 | 39.20 | 48.87 | 10.84 | 4.83 | 0 |
| moonlit | 1639 | 36.14 | 44.56 | 29.86 | 5.95 | 0 |
| orbit | 1509 | 38.33 | 50.14 | 9.10 | 7.45 | 0 |
| terminator | 1508 | 38.32 | 50.52 | 8.98 | 7.00 | 0 |
| descent | 1527 | 39.71 | 55.22 | 21.60 | 8.27 | 0 |

## light-atmo-low

| scenario | frames | interval median | interval p95 | cpu work median | render median | over 100 ms |
|---|---|---|---|---|---|---|
| warmup | 739 | 38.17 | 51.43 | 9.06 | 6.45 | 2 |
| day | 1505 | 36.24 | 48.75 | 10.95 | 4.92 | 0 |
| dusk | 1505 | 39.14 | 48.83 | 11.10 | 4.92 | 0 |
| moonlit | 1506 | 38.82 | 51.05 | 35.53 | 7.04 | 0 |
| orbit | 1507 | 38.34 | 50.81 | 9.28 | 7.59 | 0 |
| terminator | 1507 | 39.67 | 51.07 | 9.00 | 6.95 | 0 |
| descent | 1571 | 39.34 | 49.27 | 21.03 | 8.15 | 0 |

## light-atmo-high

| scenario | frames | interval median | interval p95 | cpu work median | render median | over 100 ms |
|---|---|---|---|---|---|---|
| warmup | 739 | 38.28 | 51.05 | 9.17 | 6.52 | 2 |
| day | 1504 | 39.72 | 48.84 | 10.87 | 4.95 | 0 |
| dusk | 1506 | 36.95 | 48.95 | 10.91 | 4.84 | 0 |
| moonlit | 1482 | 39.49 | 52.40 | 36.22 | 7.25 | 0 |
| orbit | 1465 | 40.94 | 44.14 | 9.72 | 7.77 | 0 |
| terminator | 1482 | 40.71 | 45.56 | 10.79 | 7.57 | 0 |
| descent | 1475 | 40.11 | 64.78 | 22.72 | 8.32 | 0 |

## light-auto

| scenario | frames | interval median | interval p95 | cpu work median | render median | over 100 ms |
|---|---|---|---|---|---|---|
| warmup | 745 | 38.08 | 52.35 | 9.51 | 6.98 | 2 |
| day | 1505 | 36.88 | 48.62 | 10.50 | 5.32 | 0 |
| dusk | 1507 | 37.75 | 48.72 | 10.66 | 5.17 | 0 |
| moonlit | 1518 | 38.87 | 47.04 | 14.88 | 5.20 | 0 |
| orbit | 1523 | 39.24 | 47.88 | 5.90 | 5.09 | 0 |
| terminator | 1515 | 39.45 | 49.37 | 6.39 | 5.27 | 0 |
| descent | 1437 | 40.45 | 57.60 | 16.70 | 8.02 | 0 |
