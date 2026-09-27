#!/usr/bin/env python3
"""Update native-commands.yaml for the PHASE 3 session migration.
- point file: of the 10 session commands to capabilities/session/commands.rs
- mark current_boundary as migrated
- add allowed_callers:"browser" + allowed_rationale for the 8 commands the
  browser capability's frontend store calls (clears NATIVE-02).
Run from repo root.
"""
import io, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
YAML = os.path.join(ROOT, "docs", "architecture", "native-boundary", "native-commands.yaml")

ALL_SESSION = [
    "session_save", "session_discard", "session_list", "session_get",
    "session_delete", "session_export", "session_restore", "flush_sessions",
    "get_session_policy", "set_session_policy",
]
# the 8 commands the browser capability's frontend store calls via bridge.<cmd>
BROWSER_CALLED = {
    "session_save", "session_list", "session_get", "session_delete",
    "session_export", "session_restore", "get_session_policy", "set_session_policy",
}
NEW_FILE = "src-tauri/src/capabilities/session/commands.rs"
BOUNDARY = "migrated to capabilities/session/commands.rs (native-physical-batch-session)"
RATIONALE = (
    "browser 能力（session 标签页 UI）经能力 public 边界调用本会话持久化命令，"
    "属已评审的窄契约；session→browser 仅经 capabilities::browser::commands::create_tab，无环。"
)

lines = io.open(YAML, "r", encoding="utf-8").read().split("\n")
out = []
i = 0
n = len(lines)
while i < n:
    line = lines[i]
    stripped = line.strip()
    # detect a command key line: "  <cmd>:" (2-space indent, not 4)
    if stripped.endswith(":") and line.startswith("  ") and not line.startswith("    "):
        cmd = stripped[:-1]
        if cmd in ALL_SESSION:
            out.append(line)
            i += 1
            in_block = True
            while i < n:
                bl = lines[i]
                bs = bl.strip()
                # stop at next command key (2-space) or top-level non-indented line
                if (bs.endswith(":") and bl.startswith("  ") and not bl.startswith("    ")) or (bs != "" and not bl.startswith("  ")):
                    break
                if bs.startswith("file:"):
                    out.append('    file: "%s"' % NEW_FILE)
                elif bs.startswith("current_boundary:"):
                    out.append('    current_boundary: "%s"' % BOUNDARY)
                elif bs.startswith("permission:"):
                    out.append(bl)
                    if cmd in BROWSER_CALLED:
                        out.append('    allowed_callers: "browser"')
                        out.append('    allowed_rationale: "%s"' % RATIONALE)
                else:
                    out.append(bl)
                i += 1
            continue
    out.append(line)
    i += 1

io.open(YAML, "w", encoding="utf-8").write("\n".join(out))
print("updated %d session commands in yaml" % len(ALL_SESSION))
