#!/usr/bin/env bash
# BrowserOS offline hang evidence collector. Non-intrusive: never signals,
# restarts, traces or attaches to a user process. No cookies or WebKit cache.
set -eu
umask 077

base="${XDG_DATA_HOME:-$HOME/.local/share}/com.jizhijiandan.mvp"
diag_dir="$base/diagnostics"
mkdir -p "$diag_dir"
out="$diag_dir/support-$(date +%Y%m%d-%H%M%S)-$$"
mkdir -m 700 "$out"
printf 'BrowserOS diagnostic capture (local only)\n' > "$out/README.txt"
printf 'Collected at: %s\n' "$(date -Ins)" >> "$out/README.txt"
printf 'No cookies, WebKit cache, clipboard, content bodies or browsing history are intentionally captured.\n' >> "$out/README.txt"
printf 'Review all files before sharing outside this computer.\n' >> "$out/README.txt"
{
  date -Ins
  uptime
  free -h
  printf '\nMemory pressure:\n'
  cat /proc/pressure/memory 2>/dev/null || true
} > "$out/system.txt"

# Exact process name, never command-line arguments (can contain URLs/tokens).
ps -eo pid,ppid,etimes,stat,pcpu,pmem,rss,vsz,comm   | awk 'NR==1 || $9=="mvp-browser-os" || $9=="WebKitWebProces" || $9=="WebKitNetworkPr"'   > "$out/processes.txt" || true

pid="$(pgrep -n -x mvp-browser-os || true)"
if [ -n "$pid" ]; then
  ps -T -p "$pid" -o tid,stat,pcpu,time,wchan:32,comm > "$out/main-threads.txt" 2>/dev/null || true
  grep -E '^(Name|State|Pid|PPid|Threads|VmRSS|VmSize|VmSwap|RssAnon|RssFile|FDSize):'     "/proc/$pid/status" > "$out/main-process.txt" 2>/dev/null || true
  for task in "/proc/$pid"/task/*; do
    [ -d "$task" ] || continue
    tid="${task##*/}"
    [ -r "$task/wchan" ] || continue
    printf '%s ' "$tid" >> "$out/main-wchans.txt"
    cat "$task/wchan" >> "$out/main-wchans.txt" 2>/dev/null || true
    printf '\n' >> "$out/main-wchans.txt"
  done
fi

# Kernel evidence is limited to memory pressure, blocked threads and graphics
# driver failures, avoiding unrelated kernel messages/device paths.
journalctl -b -k --since '90 minutes ago' --no-pager 2>/dev/null   | grep -Ei 'out of memory|oom.kill|oom-kill|blocked for more|hung task|segfault|general protection|gpu hang|amdgpu.*(fault|reset|timeout)|i915.*(hang|reset|timeout)'   | tail -n 120 > "$out/kernel-signals.txt" || true

# Only the last three session logs, sanitized. Never copy raw logs, cache,
# SQLite, cookies, page HTML, request/response bodies, localStorage, or secrets.
python3 - "$base" "$out" <<'PY'
from pathlib import Path
import re,sys
base=Path(sys.argv[1]); output=Path(sys.argv[2])
files=sorted((base/'logs').glob('session-*.log'),key=lambda p:p.stat().st_mtime,reverse=True)[:3]
for index,path in enumerate(files,1):
    lines=[]
    for raw in path.read_text(errors='replace').splitlines()[-1000:]:
        if not raw.startswith('['):continue
        if any(mark in raw for mark in ('bodyText=','text="','content=','prompt=','message="','document=','payload=','sql=')):continue
        line=re.sub(r'(?i)\b(?:https?|ftp|file)://[^\s"\'<>)]+','[URL]',raw)
        line=re.sub(r'(?i)(?<![A-Z_])(appsecret|api.?key|access.?token|authorization|cookie|password|secret|token)\s*[=:]\s*[^\s,;)}\]]+',r'\1=[REDACTED]',line)
        line=re.sub(r'/home/[^/\s]+/','/home/[USER]/',line)
        line=re.sub(r'(?i)(?:Bearer|Basic)\s+[A-Za-z0-9_+/.=-]+','[AUTH_REDACTED]',line)
        lines.append(line[:500])
    (output/f'session-{index}-redacted.log').write_text('\n'.join(lines)+'\n')
PY

# Native watchdog files only contain process/thread metrics and fixed event
# codes; include at most three recent sessions, excluding raw crash stack text.
if [ -d "$diag_dir" ]; then
  find "$diag_dir" -maxdepth 1 -name 'hang-*.jsonl' -type f -printf '%T@ %p\n'     | sort -nr | head -3 | cut -d' ' -f2-     | while IFS= read -r f; do
        [ -f "$f" ] && cp -- "$f" "$out/$(basename "$f")" || true
      done
fi
tar -C "$diag_dir" -czf "$out.tar.gz" "$(basename "$out")"
printf 'DIAGNOSTICS_LOCAL_ARCHIVE=%s\n' "$out.tar.gz"
printf 'No running BrowserOS process was stopped, signaled or restarted.\n'
