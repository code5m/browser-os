#!/usr/bin/env python3
"""M0-1.c 共享 summary.json schema 校验器（无外部依赖，python3 标准库）。

用法:
  scripts/validate-summary.py --self-test
      自检：校验仓库内 schema 文件本身是合法 JSON，并跑一个内联 fixture 用例。
  scripts/validate-summary.py <schema.json> <summary.json>
      校验 summary.json 满足 schema；满足输出 'VALID' 退出 0，否则输出错误列表退出 1。

语义（对应 scripts/GATE-CONTRACT.md）:
  - schema 的 required 字段必须存在；
  - 字段类型必须匹配 properties[field].type；
  - enum / pattern 约束生效；
  - 数组字段按 items 递归校验每个元素；
  - 对象字段按 properties 递归校验（仅校验 schema 中声明的键，其余 allowed）。
"""
import json
import os
import re
import sys

ROOT_HINT = os.environ.get("GATE_SCHEMA_ROOT", "")


def find_schema_path():
    """优先取环境变量，其次相对本文件定位（脚本在仓库 scripts/ 下）。"""
    if ROOT_HINT:
        return os.path.join(ROOT_HINT, "scripts", "schema", "m0-summary.schema.json")
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(here, "schema", "m0-summary.schema.json")


def check_value(value, spec, path, errors):
    """按 JSON Schema draft-07 子集校验单个值。spec 是 properties 的条目。"""
    typ = spec.get("type")
    if typ:
        if typ == "object":
            if not isinstance(value, dict):
                errors.append(f"{path}: expected object, got {type(value).__name__}")
                return
            sub = spec.get("properties", {})
            for key, subspec in sub.items():
                if key in value:
                    check_value(value[key], subspec, f"{path}.{key}", errors)
        elif typ == "array":
            if not isinstance(value, list):
                errors.append(f"{path}: expected array, got {type(value).__name__}")
                return
            items = spec.get("items", {})
            for i, item in enumerate(value):
                check_value(item, items, f"{path}[{i}]", errors)
        elif typ == "string":
            if not isinstance(value, str):
                errors.append(f"{path}: expected string, got {type(value).__name__}")
                return
        elif typ == "integer":
            if isinstance(value, bool) or not isinstance(value, int):
                errors.append(f"{path}: expected integer, got {type(value).__name__}")
                return
        elif typ == "boolean":
            if not isinstance(value, bool):
                errors.append(f"{path}: expected boolean, got {type(value).__name__}")
                return
        elif typ == "number":
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                errors.append(f"{path}: expected number, got {type(value).__name__}")
                return
    enums = spec.get("enum")
    if enums is not None and value not in enums:
        errors.append(f"{path}: value {value!r} not in enum {enums}")
    pat = spec.get("pattern")
    if pat and isinstance(value, str):
        if re.search(pat, value) is None:
            errors.append(f"{path}: value {value!r} does not match pattern {pat!r}")
    if "required" in spec:
        if not isinstance(value, dict):
            errors.append(f"{path}: required constraint needs object")
            return
        for key in spec["required"]:
            if key not in value:
                errors.append(f"{path}: missing required field {key!r}")


def check_semantics(data, errors):
    """校验 JSON Schema 难以表达的检查点交叉约束。"""
    if not isinstance(data, dict):
        return
    status = data.get("status")
    mode = data.get("run_mode")
    checkpoint = data.get("checkpoint")
    profile = data.get("sample_profile", {})
    artifact = data.get("artifact", {})
    profile = profile if isinstance(profile, dict) else {}
    artifact = artifact if isinstance(artifact, dict) else {}

    if status == "PASS":
        if mode != "formal":
            errors.append("status: PASS requires run_mode='formal'")
        if data.get("worktree_clean_at_start") is not True:
            errors.append("status: PASS requires a clean worktree")
        if artifact.get("profile") != "release":
            errors.append("status: PASS requires a release artifact")
        if re.fullmatch(r"[0-9a-f]{64}", artifact.get("binary_sha256", "")) is None:
            errors.append("artifact.binary_sha256: PASS requires a 64-digit lowercase SHA-256")
        binary_bytes = artifact.get("binary_bytes", 0)
        if not isinstance(binary_bytes, int) or isinstance(binary_bytes, bool) or binary_bytes <= 0:
            errors.append("artifact.binary_bytes: PASS requires a non-empty binary")
    if status == "EXPLORATORY" and mode != "smoke":
        errors.append("status: EXPLORATORY requires run_mode='smoke'")
    if mode == "smoke" and status == "PASS":
        errors.append("run_mode: smoke evidence must never be PASS")

    if checkpoint == "M0-1.a" and status == "PASS":
        if profile.get("frontend_formal_samples") != 3:
            errors.append("sample_profile.frontend_formal_samples: M0-1.a PASS requires 3")
        results = data.get("results")
        if not isinstance(results, dict) or not results:
            errors.append("results: M0-1.a PASS requires non-empty results")
        else:
            failed = [name for name, result in results.items()
                      if not isinstance(result, dict) or result.get("status") != "PASS"]
            if failed:
                errors.append("results: M0-1.a PASS contains non-PASS metrics: " + ", ".join(failed))
            frontend = results.get("frontend_build_ms", {})
            frontend = frontend if isinstance(frontend, dict) else {}
            if len(frontend.get("samples_ms", [])) != 3:
                errors.append("results.frontend_build_ms.samples_ms: M0-1.a PASS requires 3 samples")

    if checkpoint == "M0-1.b" and status == "PASS":
        expected = {
            "frontend_formal_samples": 0,
            "idle_seconds": 60,
            "resource_cycle_formal_samples": 20,
            "terminal_formal_samples": 3,
        }
        for key, value in expected.items():
            if profile.get(key) != value:
                errors.append(f"sample_profile.{key}: M0-1.b PASS requires {value}")
        if data.get("blocked"):
            errors.append("blocked: M0-1.b PASS requires no blocked metrics")
        driver = data.get("driver_selfcheck", {})
        driver = driver if isinstance(driver, dict) else {}
        measurements = data.get("measurements", {})
        measurements = measurements if isinstance(measurements, dict) else {}
        if driver.get("status") != "PASS":
            errors.append("driver_selfcheck.status: M0-1.b PASS requires PASS")
        if measurements.get("ok") is not True:
            errors.append("measurements.ok: M0-1.b PASS requires true")
        if artifact.get("matches_expected") is not True:
            errors.append("artifact.matches_expected: M0-1.b PASS requires baseline artifact match")


def validate(schema_path, summary_path):
    with open(schema_path, encoding="utf-8") as f:
        schema = json.load(f)
    with open(summary_path, encoding="utf-8") as f:
        data = json.load(f)
    errors = []
    required = schema.get("required", [])
    properties = schema.get("properties", {})
    for key in required:
        if key not in data:
            errors.append(f"missing required field: {key}")
    if isinstance(data, dict):
        for key, spec in properties.items():
            if key in data:
                check_value(data[key], spec, key, errors)
    # 顶层 type 约束
    if schema.get("type") == "object" and not isinstance(data, dict):
        errors.append("root: expected object")
    check_semantics(data, errors)
    return errors


def self_test():
    tmp = os.path.join(os.path.dirname(os.path.abspath(__file__)), "schema")
    schema = os.path.join(tmp, "m0-summary.schema.json")
    # 1. schema 本身是合法 JSON
    try:
        with open(schema, encoding="utf-8") as f:
            json.load(f)
    except (OSError, ValueError) as exc:
        print(f"FAIL: schema file not valid JSON: {exc}")
        return 1
    print("PASS: schema file is valid JSON")
    # 2. 内联 fixture：合法样本必须 VALID
    good = {
        "run_id": "20260829T000000+0800_abcdef0_release_x11",
        "timestamp": "2026-08-29T00:00:00+0800",
        "checkpoint": "M0-1.b",
        "contract_version": "V1.1",
        "script_version": "M0-1.b-3",
        "run_mode": "formal",
        "sample_profile": {
            "frontend_formal_samples": 0,
            "idle_seconds": 60,
            "resource_cycle_formal_samples": 20,
            "terminal_formal_samples": 3,
        },
        "artifact": {
            "profile": "release",
            "binary_sha256": "",
            "binary_bytes": 0,
            "expected_binary_sha256": "",
            "matches_expected": False,
        },
        "status": "BLOCKED",
        "repo_root": "/tmp/fixture",
        "worktree_clean_at_start": True,
        "git": {"commit_sha": "a" * 40, "short_sha": "abcdef0", "branch": "feature-M0-baseline"},
        "evidence": {
            "run_dir": "logs/m0-baseline/x",
            "environment": "environment.json",
            "scenario": "scenario.json",
            "summary_json": "summary.json",
            "summary_md": "summary.md",
            "sha256sums": "SHA256SUMS",
        },
        "blocked": [{"metric": "startup_ready_ms", "status": "BLOCKED", "owner": "M0-0.b"}],
        "deferred": [{"metric": "script_first_response_ms", "status": "DEFERRED(M2-4)"}],
    }
    errs = validate(schema, "/dev/stdin") if False else []
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(good, f)
        good_path = f.name
    bad_path = None
    try:
        errs = validate(schema, good_path)
        if errs:
            print("FAIL: good fixture should be VALID:")
            for e in errs:
                print("  " + e)
            return 1
        print("PASS: good fixture validates")
        # 3. 坏样本：缺 required + 非法 status
        bad = dict(good)
        bad.pop("evidence")
        bad["status"] = "BOGUS"
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(bad, f)
            bad_path = f.name
        errs = validate(schema, bad_path)
        if not errs:
            print("FAIL: bad fixture should be INVALID")
            return 1
        found_missing = any("missing required field: evidence" in e for e in errs)
        found_enum = any("not in enum" in e and "status" in e for e in errs)
        if not (found_missing and found_enum):
            print("FAIL: bad fixture errors wrong shape: " + repr(errs))
            return 1
        print("PASS: bad fixture rejected (missing evidence + bad status enum)")
        # 4. 可选字段也必须校验；smoke 证据不得伪装成 PASS
        semantic_bad = dict(good)
        semantic_bad["run_mode"] = "smoke"
        semantic_bad["status"] = "PASS"
        semantic_bad["artifact"] = {
            "profile": "release",
            "binary_sha256": "a" * 64,
            "binary_bytes": 1,
            "matches_expected": True,
        }
        semantic_bad["driver_selfcheck"] = {"status": "PASS", "detail": "fixture"}
        semantic_bad["measurements"] = {"ok": True}
        semantic_bad["blocked"] = []
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(semantic_bad, f)
            semantic_bad_path = f.name
        try:
            errs = validate(schema, semantic_bad_path)
            if not any("smoke" in error and "PASS" in error for error in errs):
                print("FAIL: smoke PASS fixture should be rejected: " + repr(errs))
                return 1
            print("PASS: semantic gate rejects smoke evidence labeled PASS")
        finally:
            os.unlink(semantic_bad_path)
        print("SELF_TEST_RESULT=ALL_PASS")
        return 0
    finally:
        os.unlink(good_path)
        if bad_path is not None:
            os.unlink(bad_path)


def main(argv):
    if len(argv) >= 2 and argv[1] == "--self-test":
        return self_test()
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    schema_path, summary_path = argv[1], argv[2]
    if not os.path.exists(schema_path):
        print(f"error: schema not found: {schema_path}", file=sys.stderr)
        return 1
    if not os.path.exists(summary_path):
        print(f"error: summary not found: {summary_path}", file=sys.stderr)
        return 1
    errs = validate(schema_path, summary_path)
    if errs:
        print("INVALID:")
        for e in errs:
            print("  " + e)
        return 1
    print("VALID")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
