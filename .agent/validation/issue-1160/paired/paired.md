# #1160 T7: paired repeats (main build vs feature build, alternating)

Medians per scenario from raw `route/frames.txt`. `exe.sha256` names both builds.

## light-fixed

| scenario | metric | before r1 | before r2 | after r1 | after r2 | mean change |
|---|---|---|---|---|---|---|
| day | interval | 39.76 | 41.03 | 39.40 | 39.84 | -1.9% |
| day | render | 4.49 | 4.87 | 4.92 | 4.91 | +5.0% |
| day | cpu | 11.00 | 10.87 | 10.97 | 11.07 | +0.8% |
| day | >100 ms | 0 | 0 | 0 | 0 | |
| dusk | interval | 35.05 | 35.79 | 39.63 | 35.55 | +6.1% |
| dusk | render | 4.50 | 4.94 | 5.06 | 5.04 | +7.0% |
| dusk | cpu | 11.32 | 11.07 | 10.80 | 10.77 | -3.7% |
| dusk | >100 ms | 0 | 0 | 0 | 0 | |
| moonlit | interval | 39.14 | 38.87 | 39.41 | 39.14 | +0.7% |
| moonlit | render | 6.29 | 7.16 | 7.24 | 7.15 | +7.0% |
| moonlit | cpu | 35.70 | 35.99 | 35.76 | 35.66 | -0.4% |
| moonlit | >100 ms | 0 | 0 | 0 | 0 | |
| orbit | interval | 34.63 | 37.91 | 34.77 | 34.79 | -4.1% |
| orbit | render | 6.15 | 7.47 | 7.55 | 7.54 | +10.8% |
| orbit | cpu | 8.00 | 9.24 | 9.40 | 9.43 | +9.2% |
| orbit | >100 ms | 0 | 0 | 0 | 0 | |
| terminator | interval | 35.23 | 38.55 | 35.19 | 35.24 | -4.5% |
| terminator | render | 6.26 | 7.17 | 7.69 | 7.63 | +14.1% |
| terminator | cpu | 9.32 | 9.27 | 10.81 | 10.79 | +16.2% |
| terminator | >100 ms | 0 | 0 | 0 | 0 | |
| descent | interval | 39.67 | 39.95 | 39.66 | 39.74 | -0.3% |
| descent | render | 6.92 | 8.31 | 8.31 | 8.26 | +8.8% |
| descent | cpu | 18.82 | 21.72 | 21.46 | 21.48 | +5.9% |
| descent | >100 ms | 0 | 0 | 0 | 0 | |

## light-auto

| scenario | metric | before r1 | after r1 | mean change |
|---|---|---|---|---|
| day | interval | 39.06 | 39.36 | +0.8% |
| day | render | 5.11 | 5.09 | -0.4% |
| day | cpu | 10.16 | 12.64 | +24.4% |
| day | >100 ms | 0 | 0 | |
| dusk | interval | 35.94 | 39.44 | +9.8% |
| dusk | render | 5.31 | 5.19 | -2.3% |
| dusk | cpu | 10.30 | 12.53 | +21.6% |
| dusk | >100 ms | 0 | 0 | |
| moonlit | interval | 39.26 | 38.80 | -1.2% |
| moonlit | render | 4.87 | 4.63 | -4.8% |
| moonlit | cpu | 11.47 | 12.10 | +5.5% |
| moonlit | >100 ms | 0 | 0 | |
| orbit | interval | 38.72 | 38.73 | +0.0% |
| orbit | render | 7.72 | 7.90 | +2.3% |
| orbit | cpu | 9.23 | 9.51 | +3.0% |
| orbit | >100 ms | 0 | 0 | |
| terminator | interval | 39.02 | 38.88 | -0.4% |
| terminator | render | 7.17 | 7.21 | +0.4% |
| terminator | cpu | 9.11 | 9.50 | +4.3% |
| terminator | >100 ms | 0 | 0 | |
| descent | interval | 41.70 | 37.99 | -8.9% |
| descent | render | 8.04 | 8.39 | +4.3% |
| descent | cpu | 16.00 | 16.75 | +4.7% |
| descent | >100 ms | 0 | 0 | |
