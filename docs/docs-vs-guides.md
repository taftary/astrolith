# Docs vs guides

Binding material lives in `docs/`; descriptive reference lives in `guides/`.
In short: a new planet fact goes to guides; a new rule or decision goes to docs.

## What lives where

- `docs/` holds the material every agent must obey: the workflow
  (`workflow.md`, the source of truth for the process), the technical rules
  (`engineering.md`, one stable `E-` ID per rule), the architecture map
  (`ARCHITECTURE.md`), the architecture decisions (`decisions/`, one ADR per
  decision), the frozen ladder and pins (`universes/`), the process references
  (`board.md`, `validator-capabilities.md`), and the program manuals that
  describe how to run a tool (`planet-lab.md`).
- `guides/` holds the hierarchical knowledge base: what the universe looks
  like at each level, how it changed through time, and the design references
  behind the game's look and controls. Guides describe; they never bind.
  Canonically, guides never contradict `docs/`; when they disagree, `docs/`
  wins and the guide is fixed.

## How to decide

- Does a reader or agent need it to do the work correctly (a rule, a gate,
  a protocol, a budget, a decision and its consequences)? It goes to `docs/`.
- Does it describe what something looks like, how it behaves in the world,
  or what informed a design choice? It goes to `guides/`, through the normal
  guides process (`research`, `document`, `refresh`), which works inside
  `guides/` only and never touches `docs/`.
- A program manual (how to run a dev tool) lives in `docs/` next to the pins;
  the visual reference for what the tool shows lives in `guides/`.

## Cross-links

- `docs/` pages may point to `guides/` for background, and `guides/` pages
  must point to `docs/` for the binding rule (ladder, pins, budgets).
- The `docs/` index (`docs/README.md`) names every file and folder directly
  under `docs/`; the `guides/` index (`guides/README.md`) names every topic.
  Neither index lists the other's contents.
