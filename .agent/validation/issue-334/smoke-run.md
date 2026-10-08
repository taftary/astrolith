# Smoke run (CONFIRMED) - issue #334

The `guides` skill and subagent were exercised once on the feature branch
`feat/334-guides-agent` against a throwaway sample topic, per Spec v1 AC11
and plan task T4. The sample was removed afterwards so `guides/` ships with
only `README.md` (AC9).

- Smoke-run commit (sample present): `32155f1ec1cd4a51ed8530bae9b1901636622346`
- Sample removal commit: `6593b1235eb679c1ba1cad330a5865293323f1e0` (`[338-T4] record smoke run, remove sample (guides ships scaffold only)`)
- Runner: `guides` subagent with the `guides` skill (session ses_ee5a5be31ffet6xrlKboO8RP5n)

At the smoke-run commit, `guides/sample-topic/` holds:

- `documents/README.md`: topic title, scope, overview, links to guide, sources, diagram
- `documents/guide.md`: how a horizontal sundial works + how to read it, `[S001]` citations, References section, SVG figure with alt text and caption
- `documents/sources.md`: S001 entry (Wikipedia "Sundial", accessed 2026-10-08, URL, claims supported, used-in links, limitations)
- `images/sundial-parts.svg`: original diagram (dial plate, gnomon, hour lines), parses as XML, no licensing concern

Validation at the smoke-run commit (by the main agent): Markdown lint clean
on the 3 sample documents, `typos` clean, SVG parses, every internal link,
image path, heading anchor (`#s001--sundial`), and source reference resolves.

## Attached Documentation Update Report (verbatim, subagent final message)

# Documentation Update Report

## Request

Smoke-run task using the `guides` skill. Topic: a horizontal garden sundial
(small, self-contained, unrelated to this project). Full workflow for target
topic path `guides/sample-topic`: resolve the topic, brief web research on how
a horizontal sundial works (gnomon, hour lines, latitude dependence), create
`documents/README.md`, `documents/guide.md`, `documents/sources.md`, draw an
original SVG diagram, validate all links, and report. No committing; working
tree changes only.

## Scope

- Target topic: `guides/sample-topic` (new top-level topic)
- Target subtopic: n/a
- Operation: Research + Documentation update + Guide + Diagram creation + Source maintenance
- Refresh mode, if applicable: n/a
- Research required: yes (new-data only; no existing topic content existed to verify)
- Files inspected: `guides/README.md`; glob of `guides/sample-topic/**/*` (confirmed absent before creation)
- Assumptions made: none material; topic creation as a new top-level topic was explicitly instructed, so no owner question was raised.

## Initial inventory

- Documents inspected: `guides/README.md` (knowledge-base structure and run conventions; the only file in `guides/`)
- Images inspected: none existed
- Child topics inspected: none existed
- Related topics inspected: none existed (no aliases or competing locations for "horizontal garden sundial")
- Existing sources inspected: none existed
- Internal links inspected: none existed prior to this run

## Research

- Research path (existing-data, new-data, or combined): **new-data**.
  Existing-data research was not applicable: the topic did not exist and there
  was no current content to verify, refactor, or correct. Existing content
  (`guides/README.md`) was inspected first, per the combined-path ordering rule.
- Sources consulted:
  - S001: "Sundial", Wikipedia contributors, Wikipedia, The Free Encyclopedia
    (consulted October 2026; accessed 2026-10-08; URL https://en.wikipedia.org/wiki/Sundial).
    Fetched and read via webfetch (HTTP 200).
  - A second source was attempted (Royal Museums Greenwich how-it-works page 404;
    Britannica sundial page 403). Both fetches failed, so the run proceeded with the
    single encyclopedia source, which the task permits ("1-2 authoritative sources").
- Main findings:
  - A sundial is a flat plate (dial) plus a gnomon casting a shadow onto the dial;
    the shadow aligns with hour lines indicating time of day.
  - The style (time-telling edge of the gnomon) must be parallel to the Earth's
    rotation axis for year-round accuracy.
  - The style's angle from horizontal equals the sundial's geographic latitude.
  - Only the style's shadow is read; inexpensive decorative dials often have
    misaligned gnomons/hour lines and cannot tell correct time.
- New information: all of the above (topic was empty).
- Confirmed information: n/a (no pre-existing claims).
- Corrected information: none.
- Conflicting information: none.
- Information rejected as irrelevant: equation-of-time, longitude/time-zone, and
  daylight-saving corrections (documented in S001 but out of scope for this small
  garden-dial sample; noted in guide.md as unapplied adjustments and in README.md scope).
- Limitations: tertiary source; layout mathematics taken from the article's own
  account, not independently derived; only horizontal-dial claims used.

## Analysis

- Confirmed issues: none in existing content (there was none).
- Risks: single-source dependence, mitigated by using only the article's basic,
  uncontroversial physical claims and recording the limitation in `sources.md`.
- Missing information: none for the requested scope.
- Contradictions: none.
- Recommendations: if the sample is later expanded, add equation-of-time/longitude
  corrections and a second museum-grade source.

## Content actions

### Kept

- `guides/README.md` — pre-existing knowledge-base index; correct and untouched.

### Created

- `guides/sample-topic/documents/README.md` — topic title, scope, overview, links.
- `guides/sample-topic/documents/guide.md` — parts, how it works, how to read it (4 steps),
  limitations, `[S001]` citations, References section, diagram with caption.
- `guides/sample-topic/documents/sources.md` — S001 entry with title, author, publisher,
  published/accessed dates, type, URL, claims supported, used-in links, limitations.
- `guides/sample-topic/images/sundial-parts.svg` — original diagram with `<title>` and `<desc>`.

No refactors, updates, extensions, merges, splits, removals, or deletions.

## Diagram changes

- Images created: `guides/sample-topic/images/sundial-parts.svg` (original drawing; no licensing concern).
- Images updated: none.
- Images removed: none.
- Documents linked to images: `guide.md` (figure with alt text and caption); `README.md` (images index link).

## Link and reference changes

- Links added: `README.md` to `./guide.md`, `./sources.md`, `../images/sundial-parts.svg`;
  `guide.md` to `../images/sundial-parts.svg`, `./sources.md#s001--sundial`;
  `sources.md` to `./guide.md`, `./README.md`, external S001 URL.
- Links updated / removed / repaired: none.
- Source references added: `[S001]` citations in `guide.md` + References section; full S001 record in `sources.md`.
- Remaining broken links: none.

## Validation

- Internal links: all resolve.
- Image paths: `guides/sample-topic/images/sundial-parts.svg` exists with correct relative paths; `.svg` allowed and preferred.
- Heading anchors: `#s001--sundial` verified against the heading text.
- Source references: `[S001]` used; full record present with accessed date 2026-10-08.
- Parent-child navigation: topic README lists child/related topics (none); `guides/README.md` untouched.
- Duplicate content: none.
- Orphaned documents: none.
- Terminology consistency: dial plate, gnomon, style, hour lines, noon line used consistently.
- External references: S001 fetched HTTP 200 during research.

## Assumptions

- Task instruction as authorization to create the new top-level topic without an owner question.
- Single successfully fetched encyclopedia source as sufficient per the task allowance.
- Northern-Hemisphere orientation in guide and diagram.

## Remaining issues

- None. No owner decision required; nothing committed, per instructions.

## Final assessment

- `completed`
- The full smoke-run workflow is done: topic resolved, existing hierarchy inspected,
  new-data research performed, three documents and one original SVG diagram created
  inside `guides/sample-topic/` only, all internal links/image paths/source references
  validated, and no files outside `guides/` touched or committed.
