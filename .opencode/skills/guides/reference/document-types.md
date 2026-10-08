# Document types

Use document types according to purpose. Do not force every topic to contain every document type. Keep research facts separate from recommendations and design decisions.

- `README.md`: topic navigation and scope.
- `research.md`: external findings.
- `analysis.md`: technical interpretation.
- `critique.md`: weaknesses, contradictions, and risks.
- `recommendations.md`: proposed improvements and alternatives.
- `specification.md`: precise requirements or system description.
- `guide.md`: practical explanation or procedure.
- `decision-log.md`: decisions, rationale, alternatives, and consequences.
- `sources.md`: external source records.

# Source management

Each topic that uses external research should contain `<topic-or-subtopic>/documents/sources.md`.

Use stable identifiers such as `S001`, `S002`, `S003`. For larger topics, use prefixed identifiers such as `GEO-S001`, `PLANET-S001`.

Each source entry should contain, when available: source identifier; title; author or organization; publisher, journal, or website; publication date; access date; source type; DOI, ISBN, standard number, or other stable identifier; URL; claims supported; relevant documents; limitations or reliability concerns.

Example:

```md
## S001 — Robust geometric predicates
- **Title:** Exact Geometric Computation
- **Author:** Author name
- **Publisher:** Organization or journal
- **Published:** 2024
- **Accessed:** 2026-10-08
- **Type:** Academic paper
- **Identifier:** DOI or other stable identifier
- **URL:** [https://example.org/source](https://example.org/source)
- **Supports:**
  - Exact predicate evaluation.
  - Handling of degenerate geometric cases.
- **Used in:**
  - [Technical specification](./specification.md)
  - [Numerical stability analysis](../numerical-stability/documents/analysis.md)
- **Limitations:**
  - Does not address GPU implementation.
```

For copied images, also record the origin and the license in the source entry (see `image-policy.md`).

Cite important claims using source identifiers: `Exact predicates reduce classification errors near geometric boundaries [S001].`

Add a references section to documents that rely on sources:

```md
## References
- [S001](./sources.md#s001--robust-geometric-predicates)
```

Do not scatter long URLs throughout documents when a stable source identifier can be used. Update existing source entries instead of creating duplicates.
