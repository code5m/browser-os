"""Deterministic MCP server used only by native context integration tests."""
import json
import sys
import time
import subprocess
import os

for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    if method == "notifications/initialized":
        continue
    if len(sys.argv) > 1 and sys.argv[1] == "timeout":
        time.sleep(5)
    result = {}
    if method == "initialize":
        result = {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "fixture", "version": "1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "echo", "inputSchema": {"type": "object"}, "annotations": {"readOnlyHint": True}}]}
    elif method == "tools/call":
        if request["params"]["name"] == "echo_env":
            result = {"content": [{"type": "text", "text": os.environ.get("FIXTURE_TOKEN", "missing")}]}
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
            continue
        if request["params"]["name"] == "descendant":
            child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
            result = {"pid": child.pid}
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
            continue
        if request["params"]["name"] == "error":
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32000, "message": "fixture error"}}), flush=True)
            continue
        result = {"content": [{"type": "text", "text": request["params"]["arguments"].get("text", "ok")} ]}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
    if len(sys.argv) > 1 and sys.argv[1] == "blocked" and method == "initialize":
        time.sleep(5)
