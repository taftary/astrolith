#!/bin/sh
cd /c/Users/PERSO/source/repos/astrolith || exit 1
T=.agent/validation/issue-1085/t5
run() { if [ "$2" = none ]; then unset ASTROLITH_PROTOTYPE; else export ASTROLITH_PROTOTYPE=$2; fi
  python scripts/validation/profile_run.py --route $T/auto-check.route --out $T/$1 --timeout 1200 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "Balanced 381b4222, AC present, observed 2026-11-04" 2>&1 | tail -1; }
run auto-base none; run auto-inflight1 inflight1; echo AUTO-DONE
