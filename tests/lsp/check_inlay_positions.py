#!/usr/bin/env python3
"""Check that inlay hints use correct positions (not line 0)"""

import subprocess
import json
from pathlib import Path

lsp_path = Path("target/release/zen-lsp")
lsp = subprocess.Popen(
    [str(lsp_path)],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

def send_request(method, params, req_id):
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }
    msg = json.dumps(request)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()

def send_notification(method, params):
    notification = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }
    msg = json.dumps(notification)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()

def read_response(req_id=None):
    """Read responses, skipping notifications until we get our response"""
    while True:
        header = b""
        while b"\r\n\r\n" not in header:
            chunk = lsp.stdout.read(1)
            if not chunk:
                return None
            header += chunk
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        msg = json.loads(body.decode())

        # If it's a notification, print and continue
        if "method" in msg and "id" not in msg:
            print(f"  [notification: {msg['method']}]")
            continue

        # If we're looking for a specific ID, check it
        if req_id is not None and msg.get("id") != req_id:
            continue

        return msg

# Initialize
send_request("initialize", {"processId": 12345, "rootUri": "file:///tmp", "capabilities": {}}, 1)
init_response = read_response(1)
print("✅ Initialized")

send_notification("initialized", {})

# Open test file with variables at different lines
test_content = """main = () i32 {
    x = 42
    y = 3.14
    z = x + 10
    return 0
}
"""

print("\n📝 Test file:")
for i, line in enumerate(test_content.split('\n')):
    print(f"  {i}: {line}")

send_notification("textDocument/didOpen", {
    "textDocument": {
        "uri": "file:///tmp/test_positions.zen",
        "languageId": "zen",
        "version": 1,
        "text": test_content
    }
})

import time
time.sleep(0.3)

# Request inlay hints
send_request("textDocument/inlayHint", {
    "textDocument": {"uri": "file:///tmp/test_positions.zen"},
    "range": {
        "start": {"line": 0, "character": 0},
        "end": {"line": 10, "character": 0}
    }
}, 2)

print("\n⏳ Waiting for inlay hints response...")
response = read_response(2)

print("\n📥 Inlay hints response:")
if "result" in response and response["result"]:
    hints = response["result"]
    print(f"Got {len(hints)} hint(s):")
    for hint in hints:
        pos = hint.get("position", {})
        line = pos.get("line", -1)
        char = pos.get("character", -1)
        label = hint.get("label", "")
        print(f"  Line {line}, Char {char}: {label}")
        if line == 0:
            print("    ❌ PROBLEM: Using line 0!")
        else:
            print(f"    ✅ Correct: Actual position")
else:
    print("  (No hints or error)")
    print(f"  Response: {json.dumps(response, indent=2)}")

lsp.terminate()

print("\n✅ Test complete")
