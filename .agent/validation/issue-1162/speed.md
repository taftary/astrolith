# #1162 T7: speed work and the open question

The full route `fixtures/water-fixed.route` (fixed Normal) was run in ABBA order: before, after, after, before. Alternating that way cancels the machine's slow drift. The plain alternating pairs in `paired/` drifted by about 12% between rounds. `fixtures/ab.sh` runs one ABBA set and `fixtures/ab_table.py` writes each `table.md` from the raw `route/frames.txt`. "before" is the f943143 program, ffc9fc87…; each set's `exe.sha256` names both programs.

| set | after build | whole-route change (`table.md`, row "all") |
|---|---|---|
| `ab2` | T6 head, first after | +11.5% |
| `ab3` | no water code in dry worlds' shader (pipeline key) | +8.8% |
| `ab6` | plus ice worlds without the water shader, cheaper lake lookup | +8.9% |
| `ab7` | plus wave clock synced only on speed changes | +6.6% |
| `ab11` | plus run-time guard (the code as committed) | +8.4% |
| `ab12` | plus re-taking materials after a water change (dropped again: no gain) | +10.6% |

## Single-world checks (one ABBA set each)
| set | route | change |
|---|---|---|
| `ab4` | desert only | +0.9% |
| `ab-ab-ice-3` | ice only | +1.0% |

## The waves alone
`ab10` compares the ab7 build ("before" columns) with the same build without the close waves ("after" columns). The whole route reads -3.4%, so the waves cost about 3 to 4%.

## Reading
- **Home:** the coast at ground, from 300 m and from orbit sits at +0 to +5% in the later sets. Standing on the sea (`home-shallow`) reads +12 to +16%: the water's colour, shine and waves.
- **Dry and ice worlds, visited first:** about +1%.
- **Dry and ice worlds, after the home seas:** +5 to +16%. I did not find the cause. Three tries changed nothing:
  - a run-time guard;
  - fewer material rewrites;
  - re-taking materials.
- **Run-to-run spread:** the whole-route change moves by about ±2% between sets of the same code, from ab7's +6.6% to ab11's +8.4%.
- **Result:** the whole route stays over the +5% limit of criterion 9. This comes back to the owner.
