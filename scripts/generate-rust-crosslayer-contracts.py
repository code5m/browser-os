#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import json
import pathlib
import re
import subprocess
import sys
from dataclasses import dataclass, asdict
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUST_ROOT = ROOT / "src-tauri" / "src"
TS_ROOT = ROOT / "src"
REGISTRY = ROOT / "docs" / "architecture" / "native-boundary" / "native-commands.yaml"
TS_OUT = ROOT / "src" / "generated" / "native-contracts.ts"
JSON_OUT = ROOT / "docs" / "architecture" / "native-boundary" / "generated-crosslayer-contracts.json"

INJECTED_TYPES = ("AppHandle", "Webview", "WebviewWindow", "Window", "State<", "Manager<")


@dataclass
class RustField:
    rust_name: str
    wire_name: str
    rust_type: str
    optional: bool


@dataclass
class RustTypeDef:
    name: str
    kind: str
    source: str
    rename_all: str | None
    fields: list[RustField]
    variants: list[str]
    unsupported: str | None = None


@dataclass
class RustCommand:
    name: str
    rust_name: str
    source: str
    rename_all: str
    params: list[RustField]
    return_type: str


def read(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8")


def strip_rust_comments(text: str) -> str:
    """Strip Rust line/block comments while preserving strings and line count."""
    out: list[str] = []
    i = 0
    block_depth = 0
    in_string = False
    escape = False
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if block_depth:
            if ch == "/" and nxt == "*":
                block_depth += 1
                out.extend((" ", " "))
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_depth -= 1
                out.extend((" ", " "))
                i += 2
                continue
            out.append("\n" if ch == "\n" else " ")
            i += 1
            continue
        if in_string:
            out.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_string = False
            i += 1
            continue
        if ch == '"':
            in_string = True
            out.append(ch)
            i += 1
            continue
        if ch == "/" and nxt == "/":
            while i < len(text) and text[i] != "\n":
                out.append(" ")
                i += 1
            continue
        if ch == "/" and nxt == "*":
            block_depth = 1
            out.extend((" ", " "))
            i += 2
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def walk_files(root: pathlib.Path, suffixes: tuple[str, ...]) -> list[pathlib.Path]:
    return sorted(p for p in root.rglob("*") if p.is_file() and p.suffix in suffixes)


def split_top_level(text: str, sep: str = ",") -> list[str]:
    out: list[str] = []
    start = 0
    round_depth = square_depth = curly_depth = angle_depth = 0
    quote: str | None = None
    escape = False
    for i, ch in enumerate(text):
        if quote:
            if escape:
                escape = False
                continue
            if ch == "\\":
                escape = True
                continue
            if ch == quote:
                quote = None
            continue
        if ch == '"':
            quote = ch
        elif ch == "(":
            round_depth += 1
        elif ch == ")":
            round_depth -= 1
        elif ch == "[":
            square_depth += 1
        elif ch == "]":
            square_depth -= 1
        elif ch == "{":
            curly_depth += 1
        elif ch == "}":
            curly_depth -= 1
        elif ch == "<":
            angle_depth += 1
        elif ch == ">":
            angle_depth = max(0, angle_depth - 1)
        elif ch == sep and round_depth == square_depth == curly_depth == angle_depth == 0:
            part = text[start:i].strip()
            if part:
                out.append(part)
            start = i + 1
    tail = text[start:].strip()
    if tail:
        out.append(tail)
    return out


def find_matching(text: str, start: int, opening: str, closing: str) -> int:
    depth = 0
    quote: str | None = None
    escape = False
    for i in range(start, len(text)):
        ch = text[i]
        if quote:
            if escape:
                escape = False
                continue
            if ch == "\\":
                escape = True
                continue
            if ch == quote:
                quote = None
            continue
        if ch == '"':
            quote = ch
        elif ch == opening:
            depth += 1
        elif ch == closing:
            depth -= 1
            if depth == 0:
                return i
    return -1


def attr_value(attrs: str, key: str) -> str | None:
    m = re.search(rf'{re.escape(key)}\s*=\s*"([^"]+)"', attrs)
    return m.group(1) if m else None


def camel(name: str) -> str:
    return re.sub(r"_([A-Za-z0-9])", lambda m: m.group(1).upper(), name)


def rename(name: str, rule: str | None) -> str:
    if not rule or rule == "camelCase":
        return camel(name)
    if rule == "snake_case":
        return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).lower()
    if rule == "PascalCase":
        value = camel(name)
        return value[:1].upper() + value[1:]
    if rule == "SCREAMING_SNAKE_CASE":
        return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).replace("-", "_").upper()
    if rule == "kebab-case":
        return rename(name, "snake_case").replace("_", "-")
    if rule == "lowercase":
        return name.lower()
    if rule == "UPPERCASE":
        return name.upper()
    return name


def normalize_rust_type(value: str) -> str:
    value = re.sub(r"\s+", " ", value).strip()
    value = re.sub(r"^&\s*'\w+\s*", "", value)
    value = re.sub(r"^&\s*", "", value)
    return value.replace("serde_json::Value", "JsonValue")


def unwrap_generic(value: str) -> tuple[str, list[str]]:
    value = normalize_rust_type(value)
    pos = value.find("<")
    if pos < 0 or not value.endswith(">"):
        return value, []
    return value[:pos].strip(), split_top_level(value[pos + 1 : -1])


def parse_rust_types(files: list[tuple[str, str]]) -> dict[str, RustTypeDef]:
    out: dict[str, RustTypeDef] = {}
    duplicates: set[str] = set()
    decl = re.compile(
        r"((?:\s*#\[[^\]]+\]\s*)+)(?:pub(?:\([^)]+\))?\s+)?(struct|enum)\s+([A-Za-z_]\w*)(\s*<[^>{]+>)?\s*",
        re.MULTILINE,
    )
    for source, content in files:
        for match in decl.finditer(content):
            attrs, kind, name, generics = match.group(1), match.group(2), match.group(3), (match.group(4) or "").strip()
            if "Serialize" not in attrs and "Deserialize" not in attrs:
                continue
            pos = match.end()
            while pos < len(content) and content[pos].isspace():
                pos += 1
            rename_all = attr_value(attrs, "rename_all")
            fields: list[RustField] = []
            variants: list[str] = []
            unsupported: str | None = None
            if generics:
                unsupported = "generic serde type " + generics
            if pos < len(content) and content[pos] == "{":
                end = find_matching(content, pos, "{", "}")
                if end < 0:
                    unsupported = "unbalanced body"
                else:
                    body = content[pos + 1 : end]
                    if kind == "struct":
                        for raw_field in split_top_level(body):
                            raw_field = raw_field.strip()
                            field_attrs: list[str] = []
                            while raw_field.startswith("#["):
                                attr_end = raw_field.find("]")
                                if attr_end < 0:
                                    break
                                field_attrs.append(raw_field[: attr_end + 1])
                                raw_field = raw_field[attr_end + 1 :].strip()
                            fm = re.match(
                                r"(?:pub(?:\([^)]+\))?\s+)?([A-Za-z_]\w*)\s*:\s*([\s\S]+)$",
                                raw_field,
                            )
                            if not fm:
                                continue
                            fattrs = " ".join(field_attrs)
                            rust_name, rust_type = fm.group(1), fm.group(2).strip()
                            if re.search(r"serde\s*\([^)]*skip", fattrs):
                                continue
                            explicit = attr_value(fattrs, "rename")
                            fields.append(
                                RustField(
                                    rust_name=rust_name,
                                    wire_name=explicit or rename(rust_name, rename_all),
                                    rust_type=rust_type,
                                    optional=normalize_rust_type(rust_type).startswith("Option<"),
                                )
                            )
                    else:
                        for raw in split_top_level(body):
                            vm = re.match(r"((?:#\[[^\]]+\]\s*)*)([A-Za-z_]\w*)\s*(.*)$", raw, re.S)
                            if not vm:
                                continue
                            if vm.group(3).strip():
                                unsupported = unsupported or "data-carrying enum"
                            variants.append(attr_value(vm.group(1) or "", "rename") or rename(vm.group(2), rename_all))
            elif pos < len(content) and content[pos] == "(":
                unsupported = unsupported or "tuple struct"
            elif pos < len(content) and content[pos] == ";":
                unsupported = unsupported or "unit struct"
            else:
                unsupported = unsupported or "unknown declaration shape"
            item = RustTypeDef(name, kind, source, rename_all, fields, variants, unsupported)
            if name in out:
                duplicates.add(name)
            else:
                out[name] = item
    for name in duplicates:
        if name in out:
            out[name].unsupported = "duplicate serde type name across modules"
    return out


def parse_commands(files: list[tuple[str, str]]) -> list[RustCommand]:
    out: list[RustCommand] = []
    marker = re.compile(
        r"#\[tauri::command([^\]]*)\]\s*(?:#\[[^\]]+\]\s*)*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)\s*\(",
        re.MULTILINE,
    )
    for source, content in files:
        for match in marker.finditer(content):
            open_pos = match.end() - 1
            close_pos = find_matching(content, open_pos, "(", ")")
            if close_pos < 0:
                continue
            attrs, rust_name = match.group(1) or "", match.group(2)
            rename_all = attr_value(attrs, "rename_all") or "camelCase"
            public_name = attr_value(attrs, "rename") or rust_name
            raw_params = content[open_pos + 1 : close_pos]
            tail = content[close_pos + 1 : close_pos + 1000]
            brace = tail.find("{")
            signature_tail = tail[:brace] if brace >= 0 else tail
            signature_tail = re.split(r"\bwhere\b", signature_tail, maxsplit=1)[0].strip()
            return_type = signature_tail[2:].strip() if signature_tail.startswith("->") else "()"
            params: list[RustField] = []
            for raw in split_top_level(raw_params):
                pm = re.match(r"(?:mut\s+)?([A-Za-z_]\w*)\s*:\s*(.+)$", raw, re.S)
                if not pm:
                    continue
                rust_param, rust_type = pm.group(1), pm.group(2).strip()
                normalized = normalize_rust_type(rust_type)
                if any(token in normalized for token in INJECTED_TYPES):
                    continue
                params.append(
                    RustField(
                        rust_name=rust_param,
                        wire_name=rename(rust_param, rename_all),
                        rust_type=rust_type,
                        optional=normalized.startswith("Option<"),
                    )
                )
            out.append(RustCommand(public_name, rust_name, source, rename_all, params, return_type))
    return sorted(out, key=lambda item: item.name)


def render_type(value: str, defs: dict[str, RustTypeDef], unverified: set[str], stack: tuple[str, ...] = ()) -> str:
    value = normalize_rust_type(value)
    if value == "()":
        return "null"
    if value in {"String", "str", "PathBuf", "Url", "url::Url", "Uuid", "uuid::Uuid"}:
        return "string"
    if value == "bool":
        return "boolean"
    if re.match(r"^(?:[ui](?:8|16|32|64|128|size)|f(?:32|64)|usize|isize)$", value):
        return "number"
    if value in {"JsonValue", "Value"}:
        return "JsonValue"
    if value.startswith("(") and value.endswith(")"):
        parts = split_top_level(value[1:-1])
        return "[" + ", ".join(render_type(x, defs, unverified, stack) for x in parts) + "]" if parts else "null"
    base, args = unwrap_generic(value)
    short = base.split("::")[-1]
    if short == "Option" and args:
        return render_type(args[0], defs, unverified, stack) + " | null"
    if short in {"Vec", "HashSet", "BTreeSet"} and args:
        return "Array<" + render_type(args[0], defs, unverified, stack) + ">"
    if short in {"Box", "Arc", "Rc", "Result"} and args:
        return render_type(args[0], defs, unverified, stack)
    if short in {"HashMap", "BTreeMap"} and len(args) >= 2:
        return "Record<string, " + render_type(args[1], defs, unverified, stack) + ">"
    if short in {"DateTime", "NaiveDateTime"}:
        return "string"
    item = defs.get(short)
    if item:
        if short in stack:
            unverified.add("recursive type " + short)
            return "unknown"
        if item.unsupported:
            unverified.add(short + ": " + item.unsupported)
            return "unknown"
        if item.kind == "struct":
            pieces = []
            for field in item.fields:
                pieces.append(
                    json.dumps(field.wire_name)
                    + ("?" if field.optional else "")
                    + ": "
                    + render_type(field.rust_type, defs, unverified, stack + (short,))
                )
            return "{ " + "; ".join(pieces) + " }"
        if item.kind == "enum":
            return " | ".join(json.dumps(v) for v in item.variants) if item.variants else "never"
    unverified.add("unmapped Rust type " + value)
    return "unknown"


def parse_generate_handlers(main_text: str) -> set[str]:
    names: set[str] = set()
    for body in re.findall(r"generate_handler!\[([\s\S]*?)\]", main_text):
        body = re.sub(r"//[^\n]*", "", body)
        for item in body.split(","):
            token = item.strip()
            if re.match(r"^\w+(?:::\w+)*$", token):
                names.add(token.split("::")[-1])
    return names


def parse_registry(text: str) -> list[dict[str, str]]:
    out: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    for line in text.splitlines():
        m = re.match(r"^  ([a-z_]\w*):\s*$", line)
        if m:
            current = {"name": m.group(1)}
            out.append(current)
            continue
        m = re.match(r"^    (file|owner|category|side_effect|resource|permission|allowed_callers):\s*(.*)$", line)
        if m and current is not None:
            current[m.group(1)] = m.group(2).strip().strip('"')
    return out


def find_matching_ts(text: str, start: int, opening: str = "(", closing: str = ")") -> int:
    depth = 0
    quote: str | None = None
    escape = False
    for i in range(start, len(text)):
        ch = text[i]
        if quote:
            if escape:
                escape = False
                continue
            if ch == "\\":
                escape = True
                continue
            if ch == quote:
                quote = None
            continue
        if ch in ("'", '"', "`"):
            quote = ch
        elif ch == opening:
            depth += 1
        elif ch == closing:
            depth -= 1
            if depth == 0:
                return i
    return -1


def object_literal_keys(value: str) -> list[str] | None:
    value = value.strip()
    if not value:
        return []
    if not (value.startswith("{") and value.endswith("}")) or "..." in value:
        return None
    keys: list[str] = []
    for part in split_top_level(value[1:-1]):
        part = part.strip()
        if not part:
            continue
        match = re.match(r"(?:[\"']([^\"']+)[\"']|([A-Za-z_$]\w*))\s*(?::|$)", part)
        if not match:
            return None
        keys.append(match.group(1) or match.group(2))
    return keys


def bridge_wrapper_name(content: str, offset: int) -> str | None:
    prefix = content[max(0, offset - 1400) : offset]
    matches = list(re.finditer(r"(?m)^\s{2}([A-Za-z_$]\w*)\s*:\s*", prefix))
    return matches[-1].group(1) if matches else None


def parse_ts_observations(files: list[tuple[str, str]]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    invokes: list[dict[str, Any]] = []
    listeners: list[dict[str, Any]] = []
    invoke_re = re.compile(r"\binvoke(?:<([^>\n]+)>)?\s*\(\s*[\"']([^\"']+)[\"']")
    listen_re = re.compile(r"\blisten(?:<([^>\n]+)>)?\s*\(\s*[\"']([^\"']+)[\"']")
    for source, content in files:
        for m in invoke_re.finditer(content):
            open_pos = content.rfind("(", m.start(), m.end())
            close_pos = find_matching_ts(content, open_pos) if open_pos >= 0 else -1
            args = split_top_level(content[open_pos + 1 : close_pos]) if close_pos > open_pos else []
            payload = args[1].strip() if len(args) > 1 else ""
            keys = object_literal_keys(payload)
            invokes.append(
                {
                    "name": m.group(2),
                    "source": source,
                    "wrapper": bridge_wrapper_name(content, m.start()) if source == "src/bridge.ts" else None,
                    "tsReturn": (m.group(1) or "unknown").strip(),
                    "payloadKind": "none" if not payload else ("object" if keys is not None else "opaque"),
                    "payloadKeys": keys,
                }
            )
        for m in listen_re.finditer(content):
            prefix = content[max(0, m.start() - 500) : m.start()]
            cleanup = "RETURNED_TO_CALLER" if source == "src/bridge.ts" and "=>" in prefix else "UNVERIFIED"
            listeners.append(
                {
                    "name": m.group(2),
                    "source": source,
                    "tsType": (m.group(1) or "unknown").strip(),
                    "cleanupStatus": cleanup,
                }
            )
    return invokes, listeners


def bridge_consumers(invokes: list[dict[str, Any]], files: list[tuple[str, str]]) -> dict[str, list[str]]:
    wrappers = sorted({x["wrapper"] for x in invokes if x.get("wrapper")})
    result: dict[str, list[str]] = {}
    for wrapper in wrappers:
        pattern = re.compile(r"\bbridge\." + re.escape(wrapper) + r"\b")
        result[wrapper] = sorted(
            source for source, content in files
            if source != "src/bridge.ts" and pattern.search(content)
        )
    return result

def classify_payload(expr: str) -> str:
    value = expr.strip()
    if not value:
        return "unknown"
    if value in {"()", "serde_json::Value::Null"}:
        return "null"
    if value in {"true", "false"}:
        return "boolean"
    if value.startswith('"') or value.startswith("'"):
        return "string"
    if re.match(r"^-?\d+(?:\.\d+)?[A-Za-z0-9_]*$", value):
        return "number"
    if "json!(" in value:
        return "JsonValue"
    return "unknown"


def parse_rust_events(files: list[tuple[str, str]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    call_re = re.compile(r"\.(emit|emit_to)\s*\(")
    for source, content in files:
        for match in call_re.finditer(content):
            kind = match.group(1)
            open_pos = match.end() - 1
            close_pos = find_matching(content, open_pos, "(", ")")
            if close_pos < 0:
                continue
            args = split_top_level(content[open_pos + 1 : close_pos])
            name_index = 0 if kind == "emit" else 1
            payload_index = name_index + 1
            if len(args) <= payload_index:
                continue
            quoted = re.match(r"^[\"']([^\"']+)[\"']$", args[name_index].strip())
            if not quoted:
                continue
            expr = args[payload_index].strip()
            out.append(
                {
                    "name": quoted.group(1),
                    "source": source,
                    "kind": kind,
                    "payloadExpr": expr[:160],
                    "payloadType": classify_payload(expr),
                }
            )
    return out


def capability_from_path(source: str) -> str | None:
    m = re.match(r"src/capabilities/([^/]+)/", source)
    return m.group(1) if m else None


def rust_return_hint(value: str) -> str | None:
    value = normalize_rust_type(value)
    base, args = unwrap_generic(value)
    short = base.split("::")[-1]
    if short == "Result" and args:
        return rust_return_hint(args[0])
    if short == "Option" and args:
        inner = rust_return_hint(args[0])
        return (inner + " | null") if inner else None
    if short == "Vec" and args:
        inner = rust_return_hint(args[0])
        return (inner + "[]") if inner else None
    if value == "()":
        return "null"
    if value in {"String", "str", "PathBuf", "Url", "url::Url"}:
        return "string"
    if value == "bool":
        return "boolean"
    if re.match(r"^(?:[ui](?:8|16|32|64|128|size)|f(?:32|64)|usize|isize)$", value):
        return "number"
    if re.match(r"^[A-Za-z_]\w*(?:::\w+)*$", value):
        return value.split("::")[-1]
    return None


def normalize_ts_hint(value: str) -> str:
    return re.sub(r"\s+", " ", value.strip()).replace("void", "null")


def parse_appstate_matrix(text: str) -> list[dict[str, str]]:
    start = text.find("### 2.5 AppState")
    if start < 0:
        return []
    end = text.find("## 3.", start)
    section = text[start : end if end >= 0 else None]
    rows: list[dict[str, str]] = []
    for line in section.splitlines():
        if not line.startswith("| `"):
            continue
        cells = [x.strip().strip("`") for x in line.strip().strip("|").split("|")]
        if len(cells) < 6 or cells[0] == "AppState 字段":
            continue
        rows.append({
            "field": cells[0],
            "rustType": cells[1],
            "semantics": cells[2],
            "owner": cells[3],
            "classification": cells[4],
            "sink": cells[5],
        })
    return rows


def build_model() -> dict[str, Any]:
    rust_files = [(p.relative_to(ROOT).as_posix(), strip_rust_comments(read(p))) for p in walk_files(RUST_ROOT, (".rs",))]
    ts_files = [(p.relative_to(ROOT).as_posix(), read(p)) for p in walk_files(TS_ROOT, (".ts", ".vue"))]
    defs = parse_rust_types(rust_files)
    commands = parse_commands(rust_files)
    handlers = parse_generate_handlers(strip_rust_comments(read(RUST_ROOT / "main.rs")))
    registry = parse_registry(read(REGISTRY))
    registry_by_name = {x["name"]: x for x in registry}
    active = [c for c in commands if c.rust_name in handlers or c.name in handlers]
    invokes, listeners = parse_ts_observations(ts_files)
    consumers = bridge_consumers(invokes, ts_files)
    events = parse_rust_events(rust_files)
    unverified: set[str] = set()
    contracts: list[dict[str, Any]] = []
    for command in active:
        local: set[str] = set()
        args = {
            p.wire_name: {
                "type": render_type(p.rust_type, defs, local),
                "required": not p.optional,
                "rust": p.rust_type,
            }
            for p in command.params
        }
        result = render_type(command.return_type, defs, local)
        for item in local:
            unverified.add(command.name + ": " + item)
        observed = [x for x in invokes if x["name"] == command.name]
        reg = registry_by_name.get(command.name) or registry_by_name.get(command.rust_name)
        expected_keys = set(args)
        required_keys = {name for name, meta in args.items() if meta["required"]}
        payload_findings: list[dict[str, Any]] = []
        for call in observed:
            keys = call.get("payloadKeys")
            if keys is None:
                continue
            actual = set(keys)
            missing_keys = sorted(required_keys - actual)
            extra_keys = sorted(actual - expected_keys)
            if missing_keys or extra_keys:
                payload_findings.append({"source": call["source"], "missing": missing_keys, "extra": extra_keys})
        return_hint = rust_return_hint(command.return_type)
        return_findings: list[dict[str, str]] = []
        if return_hint:
            expected_return = normalize_ts_hint(return_hint)
            for call in observed:
                actual_return = normalize_ts_hint(call["tsReturn"])
                if actual_return == "unknown":
                    continue
                simple = re.match(r"^(?:[A-Za-z_]\w*(?:\[\])?(?: \| null)?|string|number|boolean|null)$", actual_return)
                if simple and actual_return != expected_return:
                    return_findings.append({"source": call["source"], "expected": expected_return, "actual": actual_return})
        wrappers = sorted({x["wrapper"] for x in observed if x.get("wrapper")})
        consumer_files = sorted({p for wrapper in wrappers for p in consumers.get(wrapper, [])})
        contracts.append(
            {
                "name": command.name,
                "rustName": command.rust_name,
                "source": command.source,
                "args": args,
                "result": result,
                "returnHint": return_hint,
                "registry": None
                if not reg
                else {
                    "owner": reg.get("owner"),
                    "resource": reg.get("resource"),
                    "permission": reg.get("permission"),
                    "allowedCallers": reg.get("allowed_callers"),
                },
                "bridgeWrappers": wrappers,
                "observedFrontendFiles": sorted({x["source"] for x in observed}),
                "consumerFiles": consumer_files,
                "observedCapabilities": sorted(
                    {
                        cap
                        for cap in (capability_from_path(x) for x in consumer_files)
                        if cap is not None
                    }
                ),
                "observedTsReturnTypes": sorted({x["tsReturn"] for x in observed}),
                "payloadFindings": payload_findings,
                "returnFindings": return_findings,
            }
        )
    event_names = sorted({x["name"] for x in events} | {x["name"] for x in listeners})
    event_contracts: list[dict[str, Any]] = []
    for name in event_names:
        emits = [x for x in events if x["name"] == name]
        listens = [x for x in listeners if x["name"] == name]
        rust_types = sorted({x["payloadType"] for x in emits})
        ts_types = sorted({x["tsType"] for x in listens})
        verified = len(rust_types) == 1 and len(ts_types) == 1 and rust_types[0] == ts_types[0] and rust_types[0] != "unknown"
        event_contracts.append(
            {
                "name": name,
                "emits": emits,
                "listeners": listens,
                "rustPayloadTypes": rust_types,
                "tsPayloadTypes": ts_types,
                "payloadStatus": "VERIFIED_PRIMITIVE" if verified else ("PARTIAL" if rust_types or ts_types else "UNVERIFIED"),
            }
        )
    errors: list[str] = []
    parsed_active = {c.rust_name for c in active} | {c.name for c in active}
    missing = sorted(handlers - parsed_active)
    if missing:
        errors.append("registered command signatures not parsed: " + ",".join(missing))
    impact = [
        {
            "command": c["name"],
            "owner": (c["registry"] or {}).get("owner"),
            "resource": (c["registry"] or {}).get("resource"),
            "permission": (c["registry"] or {}).get("permission"),
            "frontendFiles": c["observedFrontendFiles"],
            "frontendCapabilities": c["observedCapabilities"],
        }
        for c in contracts
    ]
    return {
        "schemaVersion": 1,
        "source": "code-derived",
        "authorities": {
            "rustCommands": "src-tauri/src/**/*.rs #[tauri::command] + src-tauri/src/main.rs generate_handler!",
            "rustTypes": "Serde Serialize/Deserialize declarations in src-tauri/src/**/*.rs",
            "nativeGovernance": "docs/architecture/native-boundary/native-commands.yaml",
            "resourceLifecycle": "docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md",
            "frontendObservations": "src/**/*.ts|vue literal invoke/listen calls",
        },
        "counts": {
            "rustFiles": len(rust_files),
            "registeredHandlers": len(handlers),
            "parsedCommands": len(commands),
            "activeContracts": len(contracts),
            "serdeTypes": len(defs),
            "rustEventSites": len(events),
            "eventContracts": len(event_contracts),
            "unverifiedTypeEdges": len(unverified),
        },
        "commands": contracts,
        "events": event_contracts,
        "impactGraph": impact,
        "unverified": sorted(unverified),
        "limitations": [
            "Dynamic command/event names and opaque payload objects are not structurally proven.",
            "Resource lifecycle ownership is linked from the existing Native Physical Boundary Matrix rather than duplicated.",
            "Generated unknown means UNVERIFIED; it is never treated as structural equivalence PASS.",
        ],
        "errors": errors,
    }


def render_ts(report: dict[str, Any]) -> str:
    lines = [
        "/* AUTO-GENERATED by scripts/generate-rust-crosslayer-contracts.py. DO NOT EDIT. */",
        "export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };",
        "export interface NativeCommandContracts {",
    ]
    for command in report["commands"]:
        lines.append("  " + json.dumps(command["name"]) + ": {")
        lines.append("    args: {")
        for name, meta in command["args"].items():
            suffix = "" if meta["required"] else "?"
            lines.append("      " + json.dumps(name) + suffix + ": " + meta["type"] + ";")
        lines.append("    };")
        lines.append("    result: " + command["result"] + ";")
        lines.append("  };")
    lines.extend(
        [
            "}",
            'export type NativeCommandName = keyof NativeCommandContracts;',
            'export type NativeCommandArgs<K extends NativeCommandName> = NativeCommandContracts[K]["args"];',
            'export type NativeCommandResult<K extends NativeCommandName> = NativeCommandContracts[K]["result"];',
            "export type NativeEventName = "
            + (" | ".join(json.dumps(x["name"]) for x in report["events"]) if report["events"] else "never")
            + ";",
            "",
        ]
    )
    return "\n".join(lines)


def self_test() -> bool:
    fixture = [
        (
            "fixture.rs",
            '#[derive(Serialize, Deserialize)]\n#[serde(rename_all = "camelCase")]\npub struct UserInfo { pub user_id: String, pub enabled: bool }\n#[tauri::command]\npub fn load_user(user_id: String) -> Result<UserInfo, String> { todo!() }\nfn e(app: tauri::AppHandle){ app.emit("ready", true).unwrap(); }',
        )
    ]
    defs = parse_rust_types(fixture)
    commands = parse_commands(fixture)
    if len(commands) != 1 or commands[0].params[0].wire_name != "userId":
        return False
    unverified: set[str] = set()
    rendered = render_type(commands[0].return_type, defs, unverified)
    events = parse_rust_events(fixture)
    return '"userId": string' in rendered and len(events) == 1 and events[0]["payloadType"] == "boolean"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--emit-log", action="store_true")
    parser.add_argument("--diagnostic", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        ok = self_test()
        print("RUST_CROSSLAYER_CONTRACT_SELF_TEST=" + ("PASS" if ok else "FAIL"))
        return 0 if ok else 1
    report = build_model()
    ts = render_ts(report)
    json_text = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.generate:
        TS_OUT.parent.mkdir(parents=True, exist_ok=True)
        JSON_OUT.parent.mkdir(parents=True, exist_ok=True)
        TS_OUT.write_text(ts, encoding="utf-8")
        JSON_OUT.write_text(json_text, encoding="utf-8")
    if args.check:
        if not TS_OUT.exists() or TS_OUT.read_text(encoding="utf-8") != ts:
            report["errors"].append("src/generated/native-contracts.ts drift")
        if not JSON_OUT.exists() or JSON_OUT.read_text(encoding="utf-8") != json_text:
            report["errors"].append("generated-crosslayer-contracts.json drift")
    if args.emit_log:
        print("CROSSLAYER_SUMMARY=" + json.dumps(report["counts"], separators=(",", ":")))
        print("CROSSLAYER_ERRORS=" + json.dumps(report["errors"], separators=(",", ":")))
        print("CROSSLAYER_UNVERIFIED_SAMPLE=" + json.dumps(report["unverified"][:50], separators=(",", ":")))
        print("CROSSLAYER_TS_B64=" + base64.b64encode(ts.encode()).decode())
        print("CROSSLAYER_JSON_B64=" + base64.b64encode(json_text.encode()).decode())
    print(
        "RUST_CROSSLAYER_CONTRACT_RESULT="
        + ("FAIL" if report["errors"] else "PASS")
        + " commands="
        + str(report["counts"]["activeContracts"])
        + " events="
        + str(report["counts"]["eventContracts"])
        + " unverified="
        + str(report["counts"]["unverifiedTypeEdges"])
    )
    for error in report["errors"]:
        print("- " + error, file=sys.stderr)
    if args.diagnostic:
        return 0
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
