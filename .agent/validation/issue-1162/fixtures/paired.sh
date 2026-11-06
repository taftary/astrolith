#!/usr/bin/env bash
# #1162 T7: paired repeats, alternating the before build and the after build
# on the same routes, to separate run-to-run noise at the VSync lock from
# real cost (the #1160 method).
# Usage: bash .agent/validation/issue-1162/fixtures/paired.sh <before.exe> <after.exe>
set -u
before="$1"
after="$2"
D=.agent/validation/issue-1162
out="$D/paired"
mkdir -p "$out"
sha256sum "$before" "$after" > "$out/exe.sha256"
run() { # side, route, round
  cp "$([ "$1" = before ] && echo "$before" || echo "$after")" target/release/universe-app.exe
  python scripts/validation/profile_run.py --route "$D/fixtures/$2.route" --out "$out/$1-$2-r$3" --timeout 1200 --power-note "AC, Balanced" > "$out/$1-$2-r$3.log" 2>&1
  echo "$1 $2 r$3 rc=$?"
}
for round in 1 2; do
  run before water-fixed "$round"
  run after water-fixed "$round"
done
run before water-auto 1
run after water-auto 1
cp "$after" target/release/universe-app.exe
echo PAIRED-DONE
