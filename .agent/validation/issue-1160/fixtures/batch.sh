#!/usr/bin/env bash
# #1160: matched pictures plus profile routes for one side (before or after).
# Usage: bash .agent/validation/issue-1160/fixtures/batch.sh <before|after>
# Needs a fresh `cargo build --locked --release -p universe-app`.
set -u
side="$1"
D=.agent/validation/issue-1160
out="$D/$side"
mkdir -p "$out"
sha256sum target/release/universe-app.exe > "$out/exe.sha256"
git rev-parse HEAD > "$out/commit.txt"
./target/release/universe-app.exe --screens "$out/pictures" --shots "$D/fixtures/light.shots" > "$out/pictures.log" 2>&1
echo "pictures rc=$? $(tail -1 "$out/pictures.log")"
for r in light-fixed light-atmo-low light-atmo-high light-auto; do
  python scripts/validation/profile_run.py --route "$D/fixtures/$r.route" --out "$out/$r" --timeout 900 --power-note "AC, Balanced" > "$out/$r.log" 2>&1
  echo "$r rc=$?"
done
echo BATCH-DONE
