# Docs index

`docs/` holds the binding material: the process, the technical rules, the
architecture map, the decisions, and the pins. Descriptive reference lives in
`guides/`; the rule for which is which is
[docs-vs-guides.md](docs-vs-guides.md). One line per item, with its Audience.

| Item | Audience | What it is |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | AI, owner | Bird's-eye view, entry points, codemap, invariants. |
| [board.md](board.md) | AI | Project status moves and the one-glance summary. |
| [decisions/](decisions/) | AI, owner | One ADR per architecturally significant decision (0001-0030, consecutive). |
| [docs-vs-guides.md](docs-vs-guides.md) | AI, owner | The rule for what lives in `docs/` versus `guides/`. |
| [engine-lab.md](engine-lab.md) | owner, AI | The Engine lab manual: a PC-only development program showing any level L1-L10 of the game (#640, #654). |
| [engineering.md](engineering.md) | AI (owner for rule changes) | The complete technical rules, each with a stable `E-` ID. |
| [plans/](plans/) | AI | Historical plans, read-only; live plans live on their Issue. |
| [README.md](README.md) | everyone | This index. |
| [templates/](templates/) | AI | Ready-made shapes for new docs (ADR template plus when-to-use note). |
| [universes/](universes/) | AI, owner | The frozen scale ladder plus pins, features, and budgets. |
| [validator-capabilities.md](validator-capabilities.md) | validator, AI | What the validator can check, with proof per entry. |
| [workflow.md](workflow.md) | AI, owner | The process, source of truth for all stages. |
