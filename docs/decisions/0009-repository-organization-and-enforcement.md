# 0009 — Repository organization and enforcement

- Status: proposed
- Date: 2026-10-03
- Issue: #85 (flipped to accepted in M7)

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
