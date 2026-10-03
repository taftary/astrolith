# 0009 — Repository organization and enforcement

- Status: accepted
- Date: 2026-10-03
- Issue: #85 (proposed in M1, accepted in M7 with the evidence below)

## Context

Section 9 of the notion (revision notes): five review loops added
technique sections 4.9-4.13 (error handling, performance, testing, API
design, supply chain), each grounded in named sources and in repository
evidence; a closing pass aligned the goal, layout, non-goals, and owner
checks; an owner feedback pass rewrote section 4.5 so the instruction
files carry everything this work produced. The rules existed only in the
draft and, once taken, on the Issue: a new session would not see them.

## Decision

One home per rule: `docs/engineering.md` (every rule, stable `E-` ID,
CI-checked or convention), `docs/ARCHITECTURE.md` (codemap, invariants,
boundaries), `docs/decisions/` (why), `docs/README.md` (index);
`AGENTS.md` carries the always-on block and pointers; `CLAUDE.md` and
`CONTRIBUTING.md` are pointers only. Enforcement is split machine
(CI-checked) versus reviewer (convention) per rule, with the drift guard
checking sections, IDs, pointers, index, and ADR numbering
(`E-DOC-INDEX`, `E-ADR-ONE`, `E-POINTER-ONLY`, `E-AGENTS-REF`,
`E-FILE-SIZE`).

## Consequences

A fresh session recovers every rule and its origin from `AGENTS.md`,
`docs/engineering.md`, and `docs/decisions/` alone. Rules evolve in the
same pull request as the code via the template's rule question.

## Final layout (M7)

- `docs/engineering.md`: every rule with its `E-` ID and its check; the
  "Status at M1" block deleted in M2 once every check was wired.
- `docs/ARCHITECTURE.md`: final codemap (core `nav`/`labels`, render
  `camera`/`input`/`draw`/`style`, app `main`/`verify`/`capture`/`protocol`,
  golden tests); the M1 known-violation note removed in M7.
- `docs/decisions/`: ADRs 0001-0009, this record accepted in M7.
- Enforcement: workspace lints, `cargo doc`, dependency direction with the
  headless-driver import check, `cargo deny`, spell check, golden tests,
  doc tests, output vocabulary, pinned actions, permissions, concurrency,
  build cache, weekly advisories, Dependabot with pins ignored.

## Evidence (M0-M6, all on the parent Issue and sub-issues)

- M0 golden baseline: https://github.com/taftary/astrolith/issues/117#issuecomment-5968776778
- M1 documents and traceability: https://github.com/taftary/astrolith/issues/118#issuecomment-5968809559 and https://github.com/taftary/astrolith/issues/85#issuecomment-5968812095
- M2 tooling and CI: https://github.com/taftary/astrolith/issues/119#issuecomment-5969254594
- M3 core move: https://github.com/taftary/astrolith/issues/120#issuecomment-5969350084
- M4 render split: https://github.com/taftary/astrolith/issues/121#issuecomment-5969420200
- M5 app split and exit 2: https://github.com/taftary/astrolith/issues/122#issuecomment-5969465826
- M6 hygiene: https://github.com/taftary/astrolith/issues/123#issuecomment-5969493494
