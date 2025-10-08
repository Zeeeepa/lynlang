#!/usr/bin/env python3
"""Simple inlay hints test"""

import json
import subprocess
import os
import time

def send_request(proc, method, params, req_id):
    request = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

def send_notification(proc, method, params):
    request = {"jsonrpc": "2.0", "method": method, "params": params}
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

lsp = subprocess.Popen(
    ["./target/debug/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Initialize
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    line = lsp.stdout.readline().decode()
    length = int(line.split(":")[1].strip())
    lsp.stdout.readline()
    lsp.stdout.read(length)

    send_notification(lsp, "initialized", {})

    # Open document with variables
    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

main = () i32 {
    value = 42
    result = add(10, 20)
    return 0
}"""

    uri = f"file://{os.getcwd()}/test_inlay.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    time.sleep(0.3)

    # Inlay hints request
    send_request(lsp, "textDocument/inlayHint", {
        "textDocument": {"uri": uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 10, "character": 0}
        }
    }, 2)

    # Read responses
    for i in range(10):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)

        if response.get("id") == 2:
            print("✅ Got inlay hints response!")
            result = response.get("result")
            if result:
                print(f"✅ SUCCESS: {len(result)} hint(s) found!")
                for hint in result:
                    pos = hint.get("position", {})
                    label = hint.get("label")
                    kind = hint.get("kind")
                    kind_str = "TYPE" if kind == 1 else "PARAMETER" if kind == 2 else "UNKNOWN"
                    print(f"  Line {pos.get('line')}, Col {pos.get('character')}: {label} ({kind_str})")
            else:
                print("⚠️  No hints returned")
            break
        elif "method" in response:
            pass  # Skip notifications

finally:
    lsp.terminate()
    lsp.wait()
