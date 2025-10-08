#!/usr/bin/env python3
"""Test cross-file rename"""

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

def read_responses(proc, count=1):
    """Read N responses"""
    responses = []
    for _ in range(count):
        line = proc.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        proc.stdout.readline()
        content = proc.stdout.read(length).decode()
        responses.append(json.loads(content))
    return responses

lsp = subprocess.Popen(
    ["./target/debug/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Initialize
    root_uri = f"file:///tmp"
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": root_uri, "capabilities": {}}, 1)
    responses = read_responses(lsp, 1)
    print(f"✅ Init response: id={responses[0].get('id')}")

    send_notification(lsp, "initialized", {})
    time.sleep(0.1)

    # Open test file
    test_code = """greet = (name: StaticString) void {
    print("Hello, ")
    print(name)
}

main = () i32 {
    greet("World")
    return 0
}"""

    uri = f"file:///tmp/test_rename_main.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    time.sleep(0.3)

    # Rename the "greet" function (should rename both declaration and call)
    send_request(lsp, "textDocument/rename", {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 0},  # "greet" function name
        "newName": "sayHello"
    }, 2)

    # Read response
    for i in range(10):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)

        if response.get("id") == 2:
            print(f"\n✅ Got rename response!")
            if response.get("result"):
                print(f"✅ SUCCESS: Rename returned edits!")
                if "changes" in response["result"]:
                    for file_uri, edits in response["result"]["changes"].items():
                        print(f"\n  File: {file_uri}")
                        print(f"  Edits: {len(edits)}")
                        for edit in edits:
                            start = edit["range"]["start"]
                            print(f"    Line {start['line']}, Col {start['character']}: '{edit['newText']}'")
                    
                    # Verify we got 2 edits (declaration + call)
                    total_edits = sum(len(edits) for edits in response["result"]["changes"].values())
                    if total_edits >= 2:
                        print(f"\n✅ PASS: Found {total_edits} edits (declaration + call)")
                    else:
                        print(f"\n❌ FAIL: Only found {total_edits} edit(s), expected at least 2")
            else:
                print(f"\n❌ FAIL: Null result")
            break
        elif "method" in response:
            pass  # Skip notifications

finally:
    lsp.terminate()
    lsp.wait()
