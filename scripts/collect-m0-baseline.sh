#!/usr/bin/env bash
# M0-0.b/c 总控：仓库外暂存 -> 每批质量/资源证据核验 -> 聚合 -> 一次性归档。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
SCHEMA="$SCRIPT_DIR/schema/m0-summary.schema.json"
VALIDATOR="$SCRIPT_DIR/validate-summary.py"
AGGREGATOR="$SCRIPT_DIR/aggregate-m0-baseline.py"

usage() {
  cat <<'EOF'
collect-m0-baseline.sh — M0-0.b/c 原子采集总控

用法:
  scripts/collect-m0-baseline.sh                 M0-0.b：1 批完整正式采集
  scripts/collect-m0-baseline.sh --batches 3     M0-0.c：同 commit 3 批完整正式采集
  scripts/collect-m0-baseline.sh --smoke         缩短烟测；只产 EXPLORATORY 暂存证据，不归档
  scripts/collect-m0-baseline.sh --self-test     只验证总控依赖与聚合算法
  scripts/collect-m0-baseline.sh --help

可选环境变量:
  M0_STAGE_ROOT=PATH        指定仓库外暂存目录；默认 /tmp/m0-baseline-stage.*
  BS_FRONTEND_SAMPLES=N     仅 smoke 可覆盖；默认 1
  VR_CYCLE_SAMPLES=N        仅 smoke 可覆盖；默认 1
  VR_IDLE_SECONDS=N         仅 smoke 可覆盖；默认 5

正式模式固定 frontend=3、resource cycle=20、idle=60s、terminal=3；不接受缩短。
所有批次先写仓库外，完整校验后才一次性复制到 logs/m0-baseline/。
EOF
}

log() {
  echo "[M0-collector] $*"
}

find_summary() {
  local evidence_root="$1"
  python3 - "$evidence_root" <<'PY'
from pathlib import Path
import sys
matches = sorted(Path(sys.argv[1]).glob("*/summary.json"))
if len(matches) != 1:
    raise SystemExit("expected one summary.json under %s, got %d" % (sys.argv[1], len(matches)))
print(matches[0])
PY
}

verify_run() {
  local summary="$1" run_dir
  run_dir="$(dirname "$summary")"
  python3 "$VALIDATOR" "$SCHEMA" "$summary" >/dev/null
  (cd "$run_dir" && sha256sum -c SHA256SUMS >/dev/null)
}

run_self_test() {
  local rc=0 file
  for file in "$SCRIPT_DIR/baseline-check.sh" "$SCRIPT_DIR/verify-resources.sh" "$VALIDATOR" "$AGGREGATOR" "$SCHEMA"; do
    [ -f "$file" ] || { echo "FAIL: missing $file"; rc=1; }
  done
  bash -n "$SCRIPT_DIR/baseline-check.sh" || rc=1
  bash -n "$SCRIPT_DIR/verify-resources.sh" || rc=1
  python3 "$AGGREGATOR" --self-test || rc=1
  python3 "$VALIDATOR" --self-test >/dev/null || rc=1
  if [ "$rc" -eq 0 ]; then
    echo "SELF_TEST_RESULT=ALL_PASS"
  else
    echo "SELF_TEST_RESULT=FAIL"
  fi
  return "$rc"
}

MODE="formal"
BATCHES=1
while [ $# -gt 0 ]; do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    --self-test) run_self_test; exit $? ;;
    --smoke) MODE="smoke"; shift ;;
    --batches)
      [ $# -ge 2 ] || { echo "error: --batches requires 1 or 3" >&2; exit 2; }
      BATCHES="$2"
      shift 2
      ;;
    --) shift; break ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done
[ $# -eq 0 ] || { echo "error: unexpected arguments: $*" >&2; exit 2; }
case "$BATCHES" in 1|3) ;; *) echo "error: --batches must be 1 or 3" >&2; exit 2 ;; esac
if [ "$MODE" = "smoke" ] && [ "$BATCHES" -ne 1 ]; then
  echo "error: smoke mode only supports one exploratory batch" >&2
  exit 2
fi

if [ "$MODE" = "formal" ]; then
  porcelain="$(git -C "$ROOT" status --porcelain=v1)"
  if [ -n "$porcelain" ]; then
    echo "error: formal collection requires a clean worktree" >&2
    echo "$porcelain" >&2
    exit 1
  fi
fi

if [ -n "${M0_STAGE_ROOT:-}" ]; then
  STAGE_ROOT="$M0_STAGE_ROOT"
  mkdir -p "$STAGE_ROOT"
else
  STAGE_ROOT="$(mktemp -d /tmp/m0-baseline-stage.XXXXXX)"
fi
LOG_ROOT="$STAGE_ROOT/orchestrator"
mkdir -p "$LOG_ROOT"
GIT_FULL_SHA="$(git -C "$ROOT" rev-parse HEAD)"
GIT_SHORT_SHA="$(git -C "$ROOT" rev-parse --short=7 HEAD)"
EXPECTED_STATUS="PASS"
FRONTEND_SAMPLES=3
CYCLE_SAMPLES=20
IDLE_SECONDS=60
if [ "$MODE" = "smoke" ]; then
  EXPECTED_STATUS="EXPLORATORY"
  FRONTEND_SAMPLES="${BS_FRONTEND_SAMPLES:-1}"
  CYCLE_SAMPLES="${VR_CYCLE_SAMPLES:-1}"
  IDLE_SECONDS="${VR_IDLE_SECONDS:-5}"
fi

log "mode=$MODE batches=$BATCHES commit=$GIT_SHORT_SHA"
log "stage_root=$STAGE_ROOT"

for batch in $(seq 1 "$BATCHES"); do
  tag="$(printf '%02d' "$batch")"
  batch_root="$STAGE_ROOT/batch-$tag"
  quality_root="$batch_root/quality"
  resource_root="$batch_root/resources"
  mkdir -p "$quality_root" "$resource_root"

  log "batch $batch/$BATCHES: quality/build gate"
  set +e
  M0_RUN_MODE="$MODE" M0_EVIDENCE_ROOT="$quality_root" M0_EVIDENCE_LABEL_ROOT="logs/m0-baseline" \
    BS_FRONTEND_SAMPLES="$FRONTEND_SAMPLES" bash "$SCRIPT_DIR/baseline-check.sh" \
    2>&1 | tee "$LOG_ROOT/batch-${tag}-quality.log"
  quality_rc=${PIPESTATUS[0]}
  set -e
  if [ "$quality_rc" -ne 0 ]; then
    echo "error: batch $batch quality gate failed; staged evidence retained at $STAGE_ROOT" >&2
    exit 1
  fi
  quality_summary="$(find_summary "$quality_root")"
  verify_run "$quality_summary"
  quality_status="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["status"])' "$quality_summary")"
  binary_sha="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["artifact"]["binary_sha256"])' "$quality_summary")"
  if [ "$quality_status" != "$EXPECTED_STATUS" ] || ! printf '%s' "$binary_sha" | grep -Eq '^[0-9a-f]{64}$'; then
    echo "error: batch $batch quality evidence invalid: status=$quality_status sha=$binary_sha" >&2
    exit 1
  fi

  log "batch $batch/$BATCHES: resource/runtime gate"
  set +e
  M0_RUN_MODE="$MODE" M0_EVIDENCE_ROOT="$resource_root" M0_EVIDENCE_LABEL_ROOT="logs/m0-baseline" \
    M0_EXPECTED_BINARY_SHA256="$binary_sha" VR_CYCLE_SAMPLES="$CYCLE_SAMPLES" VR_IDLE_SECONDS="$IDLE_SECONDS" \
    bash "$SCRIPT_DIR/verify-resources.sh" 2>&1 | tee "$LOG_ROOT/batch-${tag}-resources.log"
  resource_rc=${PIPESTATUS[0]}
  set -e
  if [ "$resource_rc" -ne 0 ]; then
    echo "error: batch $batch resource gate failed; staged evidence retained at $STAGE_ROOT" >&2
    exit 1
  fi
  resource_summary="$(find_summary "$resource_root")"
  verify_run "$resource_summary"
  resource_status="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["status"])' "$resource_summary")"
  if [ "$resource_status" != "$EXPECTED_STATUS" ]; then
    echo "error: batch $batch resource evidence invalid: status=$resource_status" >&2
    exit 1
  fi
done

aggregate_id="$(date +%Y%m%dT%H%M%S%z)_${GIT_SHORT_SHA}_$([ "$BATCHES" -eq 3 ] && echo M0-0.c || echo M0-0.b)"
aggregate_dir="$STAGE_ROOT/aggregate/$aggregate_id"
mkdir -p "$aggregate_dir"
set +e
python3 "$AGGREGATOR" --stage-root "$STAGE_ROOT" --batches "$BATCHES" --mode "$MODE" --output-dir "$aggregate_dir"
aggregate_rc=$?
set -e
aggregate_status="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["status"])' "$aggregate_dir/summary.json" 2>/dev/null || echo FAIL)"
cp -a "$LOG_ROOT" "$aggregate_dir/logs"
(cd "$aggregate_dir" && find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 -n1 sha256sum >SHA256SUMS)

if [ "$MODE" = "formal" ] && { [ "$aggregate_status" = "PASS" ] || [ "$aggregate_status" = "UNSTABLE" ]; }; then
  archive_root="$ROOT/logs/m0-baseline"
  manifest_root="$archive_root/manifests/$aggregate_id"
  mkdir -p "$archive_root" "$ROOT/logs/m0-baseline/manifests"
  for batch in $(seq 1 "$BATCHES"); do
    tag="$(printf '%02d' "$batch")"
    for category in quality resources; do
      run_dir="$(dirname "$(find_summary "$STAGE_ROOT/batch-$tag/$category")")"
      destination="$archive_root/$(basename "$run_dir")"
      if [ -e "$destination" ]; then
        echo "error: archive destination already exists: $destination" >&2
        exit 1
      fi
      cp -a "$run_dir" "$destination"
    done
  done
  [ ! -e "$manifest_root" ] || { echo "error: manifest destination already exists: $manifest_root" >&2; exit 1; }
  cp -a "$aggregate_dir" "$manifest_root"
  log "archived formal evidence to $archive_root"
else
  log "evidence not archived; staged evidence retained at $STAGE_ROOT"
fi

log "aggregate_status=$aggregate_status"
log "aggregate_summary=$aggregate_dir/summary.json"
if [ "$aggregate_rc" -eq 0 ]; then
  exit 0
fi
exit 1
