#!/usr/bin/env python3
"""Debug document symbols"""

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
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    line = lsp.stdout.readline().decode()
    length = int(line.split(":")[1].strip())
    lsp.stdout.readline()
    lsp.stdout.read(length)
    
    send_notification(lsp, "initialized", {})
    time.sleep(0.1)

    test_code = """greet = (name: StaticString) void {
    print(name)
}

main = () i32 {
    return 0
}"""

    uri = f"file://{os.getcwd()}/test_doc_sym.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {"uri": uri, "languageId": "zen", "version": 1, "text": test_code}
    })
    time.sleep(0.3)

    print("Testing document symbols...")
    send_request(lsp, "textDocument/documentSymbol", {
        "textDocument": {"uri": uri}
    }, 2)

    for i in range(10):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)
        
        if response.get("id") == 2:
            print(f"\n✅ Response received:")
            print(json.dumps(response, indent=2))
            break

finally:
    lsp.terminate()
    lsp.wait()
