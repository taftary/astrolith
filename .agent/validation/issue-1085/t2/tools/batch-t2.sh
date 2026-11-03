#!/bin/sh
# #1085 T2 batch: alternating configurations, then GPU-timer runs.
cd /c/Users/PERSO/source/repos/astrolith || exit 1
LOG="$1"
F=scripts/validation/fixtures/profile
O=.agent/validation/issue-1085/t2
run() { # name fixture
  r=$(python scripts/validation/profile_run.py --route "$F/$2.route" --out "$O/$1" --timeout 1800 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "Balanced 381b4222, AC present, observed 2026-11-03" 2>&1 | tail -1)
  echo "$(date -u +%H:%M:%S) $1 $r" >> "$LOG"
}
run rank-normal-uncapped-r1 rank-normal-uncapped
run rank-low-r1 rank-low
run rank-low-uncapped-r1 rank-low-uncapped
for k in 2 3; do
  run rank-normal-r$k rank-normal
  run rank-normal-uncapped-r$k rank-normal-uncapped
  run rank-low-r$k rank-low
  run rank-low-uncapped-r$k rank-low-uncapped
done
export ASTROLITH_GPU_TIMERS=1
run gpu-normal-uncapped rank-normal-uncapped
run gpu-low-uncapped rank-low-uncapped
echo "$(date -u +%H:%M:%S) BATCH-DONE" >> "$LOG"
