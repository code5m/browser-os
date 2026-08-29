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
        import re
        if re.search(pat, value) is None:
            errors.append(f"{path}: value {value!r} does not match pattern {pat!r}")
    if "required" in spec:
        if not isinstance(value, dict):
            errors.append(f"{path}: required constraint needs object")
            return
        for key in spec["required"]:
            if key not in value:
                errors.append(f"{path}: missing required field {key!r}")


def validate(schema_path, summary_path):
    with open(schema_path, encoding="utf-8") as f:
        schema = json.load(f)
    with open(summary_path, encoding="utf-8") as f:
        data = json.load(f)
    errors = []
    for key in schema.get("required", []):
        if key not in data:
            errors.append(f"missing required field: {key}")
            continue
        check_value(data[key], schema.get("properties", {}).get(key, {}), key, errors)
    # 顶层 type 约束
    if schema.get("type") == "object" and not isinstance(data, dict):
        errors.append("root: expected object")
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
        "contract_version": "V1.0",
        "script_version": "M0-1.b-2",
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
        print("SELF_TEST_RESULT=ALL_PASS")
        return 0
    finally:
        os.unlink(good_path)
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
