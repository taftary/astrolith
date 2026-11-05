# #1160 T7: before vs after, per scenario

Medians from raw `route/frames.txt`; change = after/before - 1. Limit (Q6 A): interval median within +5%, no new >100 ms stalls.

## light-fixed (complete True, settings_unchanged True)

| scenario | interval med before | after | change | render med before | after | cpu med before | after | >100ms before/after |
|---|---|---|---|---|---|---|---|---|
| day | 38.76 | 36.86 | -4.9% | 4.82 | 4.91 | 10.87 | 10.88 | 0/0 |
| dusk | 39.20 | 39.19 | -0.0% | 4.83 | 5.00 | 10.84 | 11.03 | 0/0 |
| moonlit | 36.14 | 39.16 | +8.4% | 5.95 | 7.21 | 29.86 | 35.78 | 0/0 |
| orbit | 38.33 | 34.80 | -9.2% | 7.45 | 7.54 | 9.10 | 9.41 | 0/0 |
| terminator | 38.32 | 35.13 | -8.3% | 7.00 | 7.69 | 8.98 | 10.90 | 0/0 |
| descent | 39.71 | 39.74 | +0.1% | 8.27 | 8.27 | 21.60 | 21.37 | 0/0 |

## light-atmo-low (complete True, settings_unchanged True)

| scenario | interval med before | after | change | render med before | after | cpu med before | after | >100ms before/after |
|---|---|---|---|---|---|---|---|---|
| day | 36.24 | 41.12 | +13.5% | 4.92 | 4.90 | 10.95 | 10.86 | 0/0 |
| dusk | 39.14 | 39.51 | +0.9% | 4.92 | 4.82 | 11.10 | 10.86 | 0/0 |
| moonlit | 38.82 | 39.01 | +0.5% | 7.04 | 7.07 | 35.53 | 35.80 | 0/0 |
| orbit | 38.34 | 37.79 | -1.4% | 7.59 | 7.13 | 9.28 | 9.06 | 0/0 |
| terminator | 39.67 | 37.97 | -4.3% | 6.95 | 6.79 | 9.00 | 9.11 | 0/0 |
| descent | 39.34 | 39.51 | +0.4% | 8.15 | 8.24 | 21.03 | 21.55 | 0/0 |

## light-atmo-high (complete True, settings_unchanged True)

| scenario | interval med before | after | change | render med before | after | cpu med before | after | >100ms before/after |
|---|---|---|---|---|---|---|---|---|
| day | 39.72 | 35.26 | -11.2% | 4.95 | 4.83 | 10.87 | 10.97 | 0/0 |
| dusk | 36.95 | 39.23 | +6.2% | 4.84 | 4.97 | 10.91 | 10.85 | 0/0 |
| moonlit | 39.49 | 39.47 | -0.1% | 7.25 | 7.21 | 36.22 | 36.04 | 0/0 |
| orbit | 40.94 | 41.10 | +0.4% | 7.77 | 7.99 | 9.72 | 9.91 | 0/0 |
| terminator | 40.71 | 40.67 | -0.1% | 7.57 | 7.67 | 10.79 | 10.89 | 0/0 |
| descent | 40.11 | 40.25 | +0.3% | 8.32 | 8.33 | 22.72 | 22.57 | 0/0 |

## light-auto (complete True, settings_unchanged True)

| scenario | interval med before | after | change | render med before | after | cpu med before | after | >100ms before/after |
|---|---|---|---|---|---|---|---|---|
| day | 36.88 | 38.90 | +5.5% | 5.32 | 5.40 | 10.50 | 10.71 | 0/0 |
| dusk | 37.75 | 38.94 | +3.2% | 5.17 | 5.26 | 10.66 | 10.57 | 0/0 |
| moonlit | 38.87 | 41.58 | +7.0% | 5.20 | 4.89 | 14.88 | 11.79 | 0/0 |
| orbit | 39.24 | 37.97 | -3.2% | 5.09 | 7.43 | 5.90 | 9.21 | 0/0 |
| terminator | 39.45 | 38.49 | -2.4% | 5.27 | 7.05 | 6.39 | 9.14 | 0/0 |
| descent | 40.45 | 37.81 | -6.5% | 8.02 | 8.29 | 16.70 | 16.41 | 0/0 |

Worst fixed-quality interval median change: +13.5%

## Reading (T7, first pass)
- **The interval median is not a stable measure on this PC.** Frames sit on the VSync lock, which switches between about 33 and 40 ms, so the same scene moves by ±10% from run to run even when the render and CPU medians are unchanged. Example: `light-atmo-low` day reads +13.5% on the interval, while render (4.92 to 4.90) and CPU (10.95 to 10.86) are the same.
- **Render and CPU medians are flat in most fixed scenes**, within about 0.2 ms.
- **Exceptions in `light-fixed`:**
  - moonlit: render 5.95 to 7.21 and CPU 29.86 to 35.78. However, the before runs at Low and High already read 7.04/7.25 render and 35.5/36.2 CPU, so the `light-fixed` before run is likely the outlier (ground building at lat -60).
  - terminator: render 7.00 to 7.69. The High before run read 7.57.
- **Auto:** in orbit and terminator, Auto now settles on Normal terrain instead of Low (telemetry legs 7 and 8). That is better settings, and it explains the higher render there.
- **No new stalls over 100 ms in any run.**
- **Next:** paired repeats (before and after builds, alternating, at least 2 each) to separate noise from cost before judging AC6.
