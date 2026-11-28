"""Read the real Rust owner modules used by legacy static contract checks.

The facade no longer physically owns many IPC handlers. Static source checks
must inspect owner files, not treat relocation as an implementation deletion.
These readers are only used for real repository input; mutation self-tests keep
checking their original in-memory fixtures with the same rule functions.
"""
from pathlib import Path
import re


def read_native_bridge_owners(root: Path, *owner_names: str, owner_first: bool = False) -> str:
    src = root / "src-tauri" / "src"
    paths = [] if owner_first else [src / "bridge.rs"]
    for name in owner_names:
        if not name or "/" in name or "\\" in name or not name.replace("_", "").isalnum():
            raise ValueError(f"invalid Native owner identifier: {name!r}")
        paths.append(src / "bridge" / f"{name}.rs")
    if owner_first:
        paths.append(src / "bridge.rs")
    missing = [str(path) for path in paths if not path.is_file()]
    if missing:
        raise FileNotFoundError(f"Native contract owner source missing: {missing}")
    return "\n".join(path.read_text(encoding="utf-8") for path in paths)

def tab_close_owner_body(source: str, extract) -> str:
    """Follow only the actual close_tab facade; do not accept orphan cleanup code.

    extract is the caller's existing Rust function-body extractor. Legacy
    inline fixtures remain supported, but once an admitted owner exists the
    public facade must return its result without swallowing an error.
    """
    wrapper = extract(source, "close_tab")
    owner = extract(source, "close_tab_admitted")
    if owner or re.search(r"\bclose_tab_admitted\s*\(", wrapper):
        code = wrapper.strip()
        if code.startswith("{") and code.endswith("}"):
            code = code[1:-1].strip()
        if not re.fullmatch(r"close_tab_admitted\s*\(\s*app\s*,\s*id\s*,\s*None\s*\)", code):
            return ""
        return owner
    return wrapper
