#!/usr/bin/env python3
"""
Minimal test for inlay hints
"""
import subprocess
import json
import time
import os

# Start LSP server
lsp_path = "target/release/zen-lsp"
proc = subprocess.Popen(
    [lsp_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=0
)

def send_request(method, params, req_id):
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }
    message = json.dumps(request)
    content = f"Content-Length: {len(message)}\r\n\r\n{message}"
    proc.stdin.write(content)
    proc.stdin.flush()

def send_notification(method, params):
    notification = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }
    message = json.dumps(notification)
    content = f"Content-Length: {len(message)}\r\n\r\n{message}"
    proc.stdin.write(content)
    proc.stdin.flush()

def read_response():
    line = proc.stdout.readline()
    if line.startswith("Content-Length:"):
        length = int(line.split(":")[1].strip())
        proc.stdout.readline()  # Skip empty line
        response_text = proc.stdout.read(length)
        return json.loads(response_text)
    return None

# Initialize
send_request("initialize", {
    "processId": os.getpid(),
    "rootUri": f"file://{os.getcwd()}",
    "capabilities": {}
}, 1)
init_resp = read_response()
print("✅ Initialized")

send_notification("initialized", {})

# Create test file
test_code = """main = () i32 {
    x = 42
    y = 3.14
    z = x + 5
    return 0
}
"""

uri = f"file://{os.getcwd()}/tests/lsp/test_inlay_minimal.zen"

# Open document
send_notification("textDocument/didOpen", {
    "textDocument": {
        "uri": uri,
        "languageId": "zen",
        "version": 1,
        "text": test_code
    }
})

# Wait for diagnostics (discard them)
time.sleep(0.3)
while True:
    proc.stdout.flush()
    import select
    if select.select([proc.stdout], [], [], 0.1)[0]:
        resp = read_response()
        if resp and resp.get("method") == "textDocument/publishDiagnostics":
            continue
    break

# Request inlay hints
send_request("textDocument/inlayHint", {
    "textDocument": {"uri": uri},
    "range": {
        "start": {"line": 0, "character": 0},
        "end": {"line": 10, "character": 0}
    }
}, 2)

# Read response
hints_resp = read_response()
print(f"\n📊 Inlay Hints Response:")
print(json.dumps(hints_resp, indent=2))

if hints_resp and hints_resp.get("result"):
    hints = hints_resp["result"]
    if hints:
        print(f"\n✅ Got {len(hints)} hints:")
        for hint in hints:
            print(f"  - Line {hint['position']['line']}, Col {hint['position']['character']}: {hint['label']}")
    else:
        print("\n⚠️  No hints returned (empty array)")
else:
    print("\n❌ No hints result")

# Cleanup
proc.terminate()
proc.wait()
if os.path.exists("tests/lsp/test_inlay_minimal.zen"):
    os.remove("tests/lsp/test_inlay_minimal.zen")
