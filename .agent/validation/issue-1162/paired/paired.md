# #1162 T7: paired repeats in ABBA order (before, after, after, before)

Medians per scenario from raw `route/frames.txt`, written by `fixtures/compare.py`; each set's `exe.sha256` names both programs. change = mean of the two after runs / mean of the two before runs - 1.

Fixed Normal (`water-fixed`):

### `fixed`

| scenario | 1-before | 2-after | 3-after | 4-before | change | >100 ms |
|---|---|---|---|---|---|---|
| home-coast-ground | 39.58 | 37.76 | 37.60 | 39.23 | -4.4% | 0/0/0/0 |
| home-coast-low | 46.65 | 48.98 | 48.83 | 50.16 | +1.0% | 0/0/0/0 |
| home-orbit | 49.12 | 50.81 | 51.51 | 50.99 | +2.2% | 2/3/3/23 |
| home-shallow | 34.00 | 39.58 | 39.38 | 36.22 | +12.4% | 0/0/0/0 |
| ice-coast-ground | 37.03 | 41.59 | 41.59 | 39.22 | +9.1% | 1/1/1/1 |
| ice-coast-low | 35.82 | 40.63 | 40.61 | 37.75 | +10.4% | 0/0/0/0 |
| desert-coast-ground | 43.51 | 46.57 | 46.43 | 44.87 | +5.2% | 1/1/1/1 |
| desert-coast-low | 41.96 | 44.09 | 44.20 | 42.52 | +4.5% | 0/0/0/0 |
| home-again | 35.86 | 38.63 | 38.53 | 36.10 | +7.2% | 1/0/1/0 |
| ice-again | 30.03 | 33.69 | 35.17 | 29.81 | +15.1% | 1/1/1/1 |
| desert-again | 34.33 | 35.80 | 37.58 | 33.71 | +7.8% | 1/1/1/1 |
| all | 35.93 | 39.07 | 39.28 | 37.29 | +7.0% | 7/7/8/27 |

Auto (`water-auto`):

### `auto`

| scenario | 1-before | 2-after | 3-after | 4-before | change | >100 ms |
|---|---|---|---|---|---|---|
| home-coast-ground | 38.61 | 41.64 | 40.31 | 35.82 | +10.1% | 0/0/0/0 |
| home-coast-low | 39.17 | 37.76 | 38.57 | 38.77 | -2.1% | 0/0/0/0 |
| home-orbit | 45.64 | 45.88 | 46.12 | 46.41 | -0.1% | 0/0/0/0 |
| home-shallow | 27.09 | 31.02 | 31.33 | 26.68 | +15.9% | 0/0/0/0 |
| ice-coast-ground | 28.06 | 31.61 | 31.34 | 34.48 | +0.6% | 1/1/1/1 |
| ice-coast-low | 27.59 | 29.62 | 30.63 | 37.99 | -8.1% | 0/0/0/0 |
| desert-coast-ground | 32.29 | 33.86 | 34.03 | 40.00 | -6.1% | 0/1/0/0 |
| desert-coast-low | 32.75 | 34.16 | 34.19 | 38.70 | -4.3% | 0/0/0/0 |
| home-again | 34.79 | 30.52 | 38.72 | 37.15 | -3.8% | 0/0/0/0 |
| ice-again | 36.67 | 27.30 | 39.16 | 38.45 | -11.5% | 1/1/0/0 |
| desert-again | 38.30 | 28.60 | 38.64 | 38.12 | -12.0% | 0/1/0/0 |
| all | 32.94 | 31.15 | 34.17 | 35.16 | -4.1% | 2/4/1/1 |
