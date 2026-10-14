# Templates

Ready-made shapes for new repository docs. This page says when to use each
template. Templates live here, never in the folder they produce into: a
stray file under `decisions/` breaks the consecutive ADR numbering the
drift guard checks.

- [adr.md](adr.md): one architecture decision record. Use it for every
  architecturally significant decision (one ADR per decision, MADR form:
  title, Status, Date, Issue, Context, Decision, Consequences). Copy it to
  `../decisions/` as `NNNN-<slug>.md` with the next consecutive number and
  fill every heading; superseded records stay, never deleted. The binding
  rule is the ADR rule in the engineering doc.
- Future templates land here with one line each, and the
  [docs index](../README.md) names this folder.
