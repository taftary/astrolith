#!/bin/sh
# #1085 T4 (#1151): Spec v1 extension to five pairs for rank-low (ambiguous scenes).
cd /c/Users/PERSO/source/repos/astrolith || exit 1
LOG="$1"; F=scripts/validation/fixtures/profile; O=.agent/validation/issue-1085/t4
run() { # name fixture prototype
  if [ "$3" = none ]; then unset ASTROLITH_PROTOTYPE; else export ASTROLITH_PROTOTYPE=$3; fi
  r=$(python scripts/validation/profile_run.py --route "$F/$2.route" --out "$O/$1" --timeout 2400 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "Balanced 381b4222, AC present, observed 2026-11-04" 2>&1 | tail -1)
  echo "$(date -u +%H:%M:%S) $1 $r" >> "$LOG"
}
run rank-low-p4-proto rank-low inflight1; run rank-low-p4-base rank-low none
run rank-low-p5-base rank-low none; run rank-low-p5-proto rank-low inflight1
echo "$(date -u +%H:%M:%S) BATCH-DONE" >> "$LOG"
