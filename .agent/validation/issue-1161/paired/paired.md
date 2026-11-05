# #1161 T8: paired repeats (before build vs after build, alternating)

Medians per scenario from raw `route/frames.txt`, written by `fixtures/compare.py`. `exe.sha256` names both builds. Run order: before r1, after r1, before r2, after r2 (surfaces-fixed), then before and after on surfaces-auto.

## surfaces-fixed

| scenario | metric | before r1 | before r2 | after r1 | after r2 | mean change |
|---|---|---|---|---|---|---|
| home-ground | interval | 30.55 | 38.05 | 35.13 | 32.93 | -0.8% |
| home-ground | render | 6.79 | 4.93 | 8.24 | 8.13 | +39.7% |
| home-ground | cpu | 20.99 | 11.09 | 21.90 | 22.44 | +38.2% |
| home-ground | >100 ms | 0 | 0 | 0 | 0 | |
| home-low | interval | 41.96 | 44.77 | 44.88 | 42.63 | +0.9% |
| home-low | render | 6.49 | 6.56 | 8.09 | 8.00 | +23.2% |
| home-low | cpu | 13.53 | 10.71 | 15.58 | 15.89 | +29.8% |
| home-low | >100 ms | 0 | 0 | 0 | 0 | |
| home-orbit | interval | 48.69 | 46.00 | 50.25 | 48.24 | +4.0% |
| home-orbit | render | 6.61 | 8.93 | 8.51 | 8.21 | +7.6% |
| home-orbit | cpu | 7.97 | 11.00 | 10.24 | 10.07 | +7.1% |
| home-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| mercury-ground | interval | 30.46 | 34.39 | 34.90 | 32.90 | +4.5% |
| mercury-ground | render | 6.80 | 5.03 | 8.24 | 8.19 | +38.9% |
| mercury-ground | cpu | 20.63 | 11.17 | 21.47 | 22.11 | +37.0% |
| mercury-ground | >100 ms | 0 | 0 | 0 | 0 | |
| mercury-low | interval | 41.68 | 44.30 | 44.47 | 42.52 | +1.2% |
| mercury-low | render | 6.45 | 6.99 | 8.08 | 7.98 | +19.5% |
| mercury-low | cpu | 13.14 | 11.15 | 15.17 | 15.27 | +25.3% |
| mercury-low | >100 ms | 0 | 0 | 0 | 0 | |
| mercury-orbit | interval | 48.14 | 45.85 | 49.97 | 47.70 | +3.9% |
| mercury-orbit | render | 6.12 | 7.90 | 8.73 | 8.89 | +25.7% |
| mercury-orbit | cpu | 7.39 | 10.00 | 10.54 | 10.78 | +22.5% |
| mercury-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| desert-ground | interval | 30.46 | 28.07 | 35.05 | 35.72 | +20.9% |
| desert-ground | render | 6.80 | 8.38 | 8.23 | 5.55 | -9.2% |
| desert-ground | cpu | 20.60 | 23.38 | 21.38 | 11.42 | -25.4% |
| desert-ground | >100 ms | 0 | 0 | 0 | 0 | |
| desert-low | interval | 42.09 | 40.46 | 44.48 | 42.48 | +5.3% |
| desert-low | render | 6.64 | 8.01 | 8.08 | 7.82 | +8.5% |
| desert-low | cpu | 13.18 | 15.78 | 14.98 | 14.32 | +1.2% |
| desert-low | >100 ms | 0 | 0 | 0 | 0 | |
| desert-orbit | interval | 48.68 | 46.22 | 50.40 | 48.19 | +3.9% |
| desert-orbit | render | 6.16 | 8.32 | 8.74 | 9.24 | +24.2% |
| desert-orbit | cpu | 7.41 | 10.27 | 10.56 | 11.13 | +22.7% |
| desert-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| iron-ground | interval | 30.53 | 34.66 | 35.40 | 32.99 | +4.9% |
| iron-ground | render | 6.79 | 5.05 | 8.30 | 8.16 | +39.0% |
| iron-ground | cpu | 20.55 | 11.28 | 21.82 | 22.04 | +37.8% |
| iron-ground | >100 ms | 0 | 0 | 0 | 0 | |
| iron-low | interval | 41.83 | 44.74 | 45.48 | 42.28 | +1.4% |
| iron-low | render | 6.49 | 6.69 | 8.19 | 7.98 | +22.7% |
| iron-low | cpu | 13.06 | 10.39 | 15.24 | 15.33 | +30.4% |
| iron-low | >100 ms | 0 | 0 | 0 | 0 | |
| iron-orbit | interval | 48.78 | 46.20 | 50.52 | 48.22 | +3.9% |
| iron-orbit | render | 6.22 | 8.83 | 8.45 | 9.58 | +19.8% |
| iron-orbit | cpu | 7.50 | 11.01 | 10.39 | 12.02 | +21.0% |
| iron-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| ice-ground | interval | 26.97 | 34.53 | 31.78 | 34.23 | +7.3% |
| ice-ground | render | 7.01 | 4.93 | 8.22 | 5.13 | +11.8% |
| ice-ground | cpu | 21.29 | 10.87 | 22.40 | 11.21 | +4.5% |
| ice-ground | >100 ms | 0 | 0 | 0 | 0 | |
| ice-low | interval | 35.93 | 39.43 | 38.29 | 39.50 | +3.2% |
| ice-low | render | 6.59 | 5.88 | 7.94 | 6.80 | +18.2% |
| ice-low | cpu | 13.64 | 9.77 | 14.40 | 10.91 | +8.1% |
| ice-low | >100 ms | 0 | 0 | 1 | 0 | |
| ice-orbit | interval | 39.08 | 38.88 | 38.87 | 39.84 | +1.0% |
| ice-orbit | render | 6.10 | 7.77 | 8.90 | 8.54 | +25.8% |
| ice-orbit | cpu | 7.25 | 9.81 | 10.90 | 10.88 | +27.7% |
| ice-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| lava-ground | interval | 29.90 | 41.79 | 39.43 | 39.92 | +10.7% |
| lava-ground | render | 6.78 | 4.99 | 5.46 | 5.49 | -6.9% |
| lava-ground | cpu | 20.73 | 10.97 | 11.20 | 11.71 | -27.7% |
| lava-ground | >100 ms | 0 | 0 | 0 | 0 | |
| lava-low | interval | 40.28 | 44.51 | 41.10 | 40.99 | -3.2% |
| lava-low | render | 6.49 | 6.52 | 7.80 | 7.82 | +20.1% |
| lava-low | cpu | 13.18 | 10.59 | 14.07 | 14.41 | +19.8% |
| lava-low | >100 ms | 0 | 0 | 0 | 0 | |
| lava-orbit | interval | 47.98 | 45.32 | 47.13 | 47.20 | +1.1% |
| lava-orbit | render | 6.07 | 8.31 | 9.15 | 9.06 | +26.6% |
| lava-orbit | cpu | 7.35 | 10.44 | 10.57 | 10.81 | +20.2% |
| lava-orbit | >100 ms | 0 | 0 | 0 | 0 | |
| all | interval | 32.28 | 43.72 | 36.41 | 39.73 | +0.2% |
| all | render | 6.69 | 6.92 | 8.18 | 7.95 | +18.5% |
| all | cpu | 19.78 | 11.41 | 16.67 | 13.93 | -1.9% |
| all | >100 ms | 0 | 0 | 1 | 0 | |

## surfaces-auto

| scenario | metric | before r1 | after r1 | mean change |
|---|---|---|---|---|
| home-ground | interval | 38.61 | 35.53 | -8.0% |
| home-ground | render | 5.31 | 5.58 | +5.1% |
| home-ground | cpu | 10.70 | 10.73 | +0.3% |
| home-ground | >100 ms | 0 | 0 | |
| home-low | interval | 37.52 | 38.52 | +2.7% |
| home-low | render | 5.94 | 6.25 | +5.2% |
| home-low | cpu | 9.12 | 9.41 | +3.2% |
| home-low | >100 ms | 0 | 0 | |
| home-orbit | interval | 38.36 | 38.35 | -0.0% |
| home-orbit | render | 8.28 | 7.88 | -4.8% |
| home-orbit | cpu | 9.91 | 9.38 | -5.3% |
| home-orbit | >100 ms | 0 | 0 | |
| mercury-ground | interval | 39.10 | 41.32 | +5.7% |
| mercury-ground | render | 5.35 | 5.25 | -1.9% |
| mercury-ground | cpu | 11.01 | 10.61 | -3.6% |
| mercury-ground | >100 ms | 0 | 0 | |
| mercury-low | interval | 38.36 | 39.43 | +2.8% |
| mercury-low | render | 6.45 | 6.85 | +6.3% |
| mercury-low | cpu | 9.54 | 10.02 | +5.0% |
| mercury-low | >100 ms | 0 | 0 | |
| mercury-orbit | interval | 41.41 | 39.09 | -5.6% |
| mercury-orbit | render | 7.71 | 8.03 | +4.1% |
| mercury-orbit | cpu | 9.70 | 10.00 | +3.1% |
| mercury-orbit | >100 ms | 0 | 0 | |
| desert-ground | interval | 35.33 | 38.44 | +8.8% |
| desert-ground | render | 5.08 | 5.71 | +12.5% |
| desert-ground | cpu | 11.05 | 10.73 | -2.9% |
| desert-ground | >100 ms | 0 | 0 | |
| desert-low | interval | 43.52 | 40.96 | -5.9% |
| desert-low | render | 6.37 | 6.62 | +4.1% |
| desert-low | cpu | 10.42 | 9.68 | -7.1% |
| desert-low | >100 ms | 0 | 0 | |
| desert-orbit | interval | 41.81 | 37.88 | -9.4% |
| desert-orbit | render | 7.80 | 8.13 | +4.2% |
| desert-orbit | cpu | 9.82 | 10.40 | +5.9% |
| desert-orbit | >100 ms | 0 | 0 | |
| iron-ground | interval | 35.45 | 41.16 | +16.1% |
| iron-ground | render | 5.04 | 5.30 | +5.1% |
| iron-ground | cpu | 10.78 | 10.73 | -0.5% |
| iron-ground | >100 ms | 0 | 0 | |
| iron-low | interval | 43.09 | 40.72 | -5.5% |
| iron-low | render | 6.22 | 7.09 | +13.9% |
| iron-low | cpu | 10.00 | 10.26 | +2.7% |
| iron-low | >100 ms | 0 | 0 | |
| iron-orbit | interval | 43.40 | 37.89 | -12.7% |
| iron-orbit | render | 8.08 | 8.15 | +0.9% |
| iron-orbit | cpu | 9.98 | 10.23 | +2.5% |
| iron-orbit | >100 ms | 0 | 0 | |
| ice-ground | interval | 25.38 | 39.00 | +53.7% |
| ice-ground | render | 8.49 | 5.29 | -37.7% |
| ice-ground | cpu | 21.77 | 10.59 | -51.4% |
| ice-ground | >100 ms | 0 | 0 | |
| ice-low | interval | 39.46 | 36.84 | -6.6% |
| ice-low | render | 5.88 | 6.13 | +4.3% |
| ice-low | cpu | 9.15 | 9.09 | -0.7% |
| ice-low | >100 ms | 0 | 0 | |
| ice-orbit | interval | 38.67 | 39.00 | +0.9% |
| ice-orbit | render | 7.26 | 7.66 | +5.5% |
| ice-orbit | cpu | 9.09 | 9.30 | +2.4% |
| ice-orbit | >100 ms | 0 | 0 | |
| lava-ground | interval | 42.55 | 39.90 | -6.2% |
| lava-ground | render | 4.95 | 5.26 | +6.3% |
| lava-ground | cpu | 10.59 | 11.27 | +6.5% |
| lava-ground | >100 ms | 0 | 0 | |
| lava-low | interval | 42.75 | 43.77 | +2.4% |
| lava-low | render | 5.83 | 7.21 | +23.7% |
| lava-low | cpu | 9.59 | 11.20 | +16.7% |
| lava-low | >100 ms | 0 | 0 | |
| lava-orbit | interval | 42.72 | 43.48 | +1.8% |
| lava-orbit | render | 7.87 | 7.63 | -3.0% |
| lava-orbit | cpu | 9.84 | 9.70 | -1.4% |
| lava-orbit | >100 ms | 0 | 0 | |
| all | interval | 41.19 | 42.21 | +2.5% |
| all | render | 6.36 | 6.19 | -2.7% |
| all | cpu | 10.46 | 10.26 | -1.9% |
| all | >100 ms | 0 | 0 | |
