#!/usr/bin/env python3
"""Simple rename test"""

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
        proc.stdout.readline()  # empty line
        content = proc.stdout.read(length).decode()
        responses.append(json.loads(content))
    return responses

lsp = subprocess.Popen(
    ["./target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL  # Ignore stderr
)

try:
    # Initialize
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    responses = read_responses(lsp, 1)
    print(f"✅ Init response: id={responses[0].get('id')}")

    # Initialized notification
    send_notification(lsp, "initialized", {})

    # Open document
    test_code = """main = () i32 {
    value = 42
    result = value + 1
    return result
}"""

    uri = f"file://{os.getcwd()}/test_rename.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    time.sleep(0.2)  # Give it time to process

    # Rename request
    send_request(lsp, "textDocument/rename", {
        "textDocument": {"uri": uri},
        "position": {"line": 1, "character": 4},  # "value"
        "newName": "myValue"
    }, 2)

    # Read response (might be diagnostic notification first, then our response)
    for i in range(5):  # Try reading up to 5 messages
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)

        if response.get("id") == 2:
            print(f"\n✅ Got rename response!")
            print(json.dumps(response, indent=2))
            if response.get("result"):
                print("\n✅ SUCCESS: Rename returned edits!")
                if "changes" in response["result"]:
                    for uri, edits in response["result"]["changes"].items():
                        print(f"  {len(edits)} edits in {uri}")
            else:
                print("\n❌ FAIL: Null result")
            break
        elif "method" in response:
            print(f"📬 Notification: {response.get('method')}")
    else:
        print("❌ Didn't get response")

finally:
    lsp.terminate()
    lsp.wait()
