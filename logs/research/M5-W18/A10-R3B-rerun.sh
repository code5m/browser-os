#!/usr/bin/env bash
# A10-R3B one-click re-run: closure gate + self-containment gate + self-tests.
# Usage: bash logs/research/M5-W18/A10-R3B-rerun.sh
# Exit 0 only if BOTH gates are clean and their self-tests pass.
set -u
# This script lives at <repo>/logs/research/M5-W18/A10-R3B-rerun.sh
R3="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$R3/../../.." && pwd)"

echo "===== [1/3] closure audit self-test ====="
python3 "$R3/A10-R3B-closure-audit.py" --self-test || { echo "SELF_TEST FAILED"; exit 1; }

echo
echo "===== [2/3] self-containment gate self-test ====="
python3 "$R3/A10-R3-prototype-selfcontainment-check.py" --self-test || { echo "SELFCONTAIN_SELF_TEST FAILED"; exit 1; }

echo
echo "===== [3/3] closure audit (real) + self-containment gate (real) ====="
python3 "$R3/A10-R3B-closure-audit.py"
CLOSURE_RC=$?
python3 "$R3/A10-R3-prototype-selfcontainment-check.py"
SELFCON_RC=$?

echo
echo "===== [4/4] emit machine-readable artifacts (JSON + per-lane guide) ====="
python3 "$R3/A10-R3B-closure-audit.py" --json > "$R3/A10-R3B-findings.json" 2>/dev/null \
  && python3 "$R3/A10-R3B-gen-remediation.py" "$R3/A10-R3B-findings.json" "$R3/A10-R3B-remediation-guide.md" \
  && echo "wrote A10-R3B-findings.json + A10-R3B-remediation-guide.md"

if [ "$CLOSURE_RC" -eq 0 ] && [ "$SELFCON_RC" -eq 0 ]; then
  echo
  echo "GATE: PASS — R3B closure gate clean."
  exit 0
else
  echo
  echo "GATE: FAIL — closure_rc=$CLOSURE_RC selfcontain_rc=$SELFCON_RC"
  exit 1
fi
