# astrolith

Astrolith is a repository run by an AI-driven GitHub workflow: the owner submits notions, feedback, and bugs, the AI specifies, plans, implements, and validates the work, and the owner tests the result. See [docs/workflow.md](docs/workflow.md).

## Run the universe window

```sh
cargo run -p universe-app              # window: nested dive L1 -> L11
cargo run -p universe-app -- --verify  # headless: replay the journey, PASS per level
```

In the window: hover a dot to highlight it, click to target it, scroll (or ArrowUp/ArrowDown) to dive toward it; it opens into the next dimension where it was. Scroll out to collapse it back. Spacebar runs the autopilot to the planet; `Esc` quits. Ladder and navigation rules: [docs/universes/ladder.md](docs/universes/ladder.md).

## Agent tools (issue sidebar, validation, backfill)

Status option IDs and the lifecycle mapping live in `.agent/project-config.json` — scripts read it, never hardcode IDs.

```sh
# Projects: read and set Status (verified by re-read, retry once, never silent)
python scripts/sidebar/project.py get --issue 53
python scripts/sidebar/project.py set-status --issue 53 --status "In progress"

# Milestones: show ("No milestone" if empty), set, clear, list open, create if missing
python scripts/sidebar/milestone.py show --issue 53
python scripts/sidebar/milestone.py set --issue 53 --title "M1"
python scripts/sidebar/milestone.py list-open

# Relationships: list ("None yet" if empty), attach sub-issues, blocked-by deps
python scripts/sidebar/relationships.py list --issue 53
python scripts/sidebar/relationships.py add-sub --parent 53 --sub 51

# Development: linked branches/PRs, create issue branch, PR body lines
python scripts/sidebar/development.py list --issue 53
python scripts/sidebar/development.py pr-body-line --issue 53 --kind parent  # Related to #53, never Closes

# Notifications: state, subscribe / unsubscribe / ignore (needs --confirm)
python scripts/sidebar/notifications.py show --issue 53

# Validation: runtime (app actually launched, logs scanned) + requirements
python scripts/validation/validate.py --issue 53 --sha <head-sha>

# Backfill: dry-run first, then pilot after approval, then apply; resume/rollback
python scripts/backfill/backfill.py --dry-run --scope open
python scripts/backfill/backfill.py --dry-run --scope recent --validate
python scripts/backfill/backfill.py --pilot
python scripts/backfill/backfill.py --apply
python scripts/backfill/backfill.py --rollback --backup .agent/backfill/backfill-before-<date>.json
```
