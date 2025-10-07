#!/usr/bin/env python3
"""Simple signature help test"""

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
    ["./target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Initialize
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    # Read init response
    line = lsp.stdout.readline().decode()
    length = int(line.split(":")[1].strip())
    lsp.stdout.readline()
    lsp.stdout.read(length)

    # Initialized notification
    send_notification(lsp, "initialized", {})

    # Open document with a function
    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

main = () i32 {
    result = add(
    return 0
}"""

    uri = f"file://{os.getcwd()}/test_sig.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    time.sleep(0.2)

    # Signature help request - cursor inside add( call
    send_request(lsp, "textDocument/signatureHelp", {
        "textDocument": {"uri": uri},
        "position": {"line": 5, "character": 17}  # Inside add(
    }, 2)

    # Read responses
    for i in range(5):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)

        if response.get("id") == 2:
            print("✅ Got signature help response!")
            print(json.dumps(response, indent=2))

            result = response.get("result")
            if result and result.get("signatures"):
                print(f"\n✅ SUCCESS: {len(result['signatures'])} signature(s) found!")
                for sig in result["signatures"]:
                    print(f"  📝 {sig.get('label')}")
            else:
                print("\n⚠️  Empty signatures")
            break
        elif "method" in response:
            print(f"📬 Notification: {response.get('method')}")

finally:
    lsp.terminate()
    lsp.wait()
    # Try to read stderr
    lsp.stderr.seek(0)
    try:
        err_data = lsp.stderr.read(10000).decode()
        if err_data:
            print("\n=== LSP Debug Output ===")
            print(err_data)
    except:
        pass
