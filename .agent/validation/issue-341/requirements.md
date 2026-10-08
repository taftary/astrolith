# Requirements — Issue #341 (Spec v2)

Source: Spec v2 comment on #341 (supersedes Spec v1). One row per
acceptance criterion with a validator probe from
`docs/validator-capabilities.md`.

## AC1 — Universe overview index

Criterion: `guides/universe/documents/README.md` exists with title,
scope, overview, links to the realism review, to each level L01-L14, to
important pictures, and to `docs/universes/ladder.md` and
`docs/universes/stack.md`.

Probe: read a rendered file on GitHub at a commit SHA (#30) plus read a
blob at a commit SHA (`git show SHA:path`, #97). Open the overview at the
PR head SHA and check each section and link target exists.

## AC2 — Level folders L01-L14

Criterion: each level L01-L14 exists as `guides/universe/L01/documents/`
through `guides/universe/L14/documents/`, each with its own `README.md`
and its moved pages, plus `guides/universe/L<n>/images/` with the same
pictures.

Probe: `git show SHA:path` for each level README (#97) and the compare
API per-file patch read (#87) to confirm the tree shape at the head SHA.

## AC3 — Nothing lost

Criterion: all 127 level pages and all 115 pictures are present under
`guides/universe` and readable; nothing is lost. The realism review is at
`guides/universe/documents/realism-review.md`. (Implementation note: the
source tree held 126 pages under `L01`-`L14` plus `levels/README.md`,
which was folded into the overview; the PR records exact counts.)

Probe: tell a GitHub-level if plus read a per-file patch out of the
compare API (#87) plus `git show` for counts; fetch a binary over HTTPS
to a local Temp file (#134) for picture readability sampling.

## AC4 — Ladder and stack stay canonical

Criterion: `docs/universes/` keeps `ladder.md`, `stack.md`, and a short
`README.md` pointing to `guides/universe`; `docs/universes/levels/` no
longer exists.

Probe: read a rendered file at the head SHA (#30) for the pointer README;
`git show SHA:path` plus compare-API patch read (#87) to confirm
`levels/` is absent and only the three files remain.

## AC5 — Docs side and script

Criterion: `docs/ARCHITECTURE.md` level links, `docs/README.md` index
line, and `scripts/validation/review_lint.py` default path point to the
new places; `review_lint` still passes on the moved file.

Probe: run python repo scripts locally (#76) plus the allowlisted-path
runnable proof (#88): `python scripts/validation/review_lint.py` prints
`REVIEW-LINT-OK` with no argument (default path is the moved file).

## AC6 — Links, orphans, lint, spelling

Criterion: every internal link, picture link, and parent-child index link
under `guides/universe` resolves; no orphan pages; Markdown lint and
spelling check pass.

Probe: byte-compare and first-differing-line diff a remote file read at
two commits, client-side, with no shell (#87) plus `git show` for link
targets (local audit script reports zero broken links and zero orphans);
pull-request check-runs for Markdown lint and typos are green (#62, #87).

## AC7 — Guides process works afterwards

Criterion: after the move, a research, document, or refresh request on
`guides/universe` works through the normal guides process.

Probe: docs-only structural check — `guides/README.md` documents the
`documents/` plus `images/` layout and the `chore/guides-<slug>` flow;
the moved tree follows that layout (audit confirms), so a follow-up
guides run has a valid target. No new agent or CI change in this item.
