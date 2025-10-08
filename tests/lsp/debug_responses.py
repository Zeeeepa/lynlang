#!/usr/bin/env python3
"""Debug LSP responses to see what's actually being returned"""

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

def read_response(proc, req_id, timeout=2.0):
    start = time.time()
    while time.time() - start < timeout:
        try:
            line = proc.stdout.readline().decode()
            if not line.startswith("Content-Length:"):
                continue
            length = int(line.split(":")[1].strip())
            proc.stdout.readline()
            content = proc.stdout.read(length).decode()
            response = json.loads(content)

            if response.get("id") == req_id:
                return response
        except Exception as e:
            print(f"Error reading: {e}")
            pass
    return None

lsp = subprocess.Popen(
    ["./target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Initialize
    send_request(lsp, "initialize", {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    }, 1)

    resp = read_response(lsp, 1, timeout=3.0)
    print("INITIALIZE:", json.dumps(resp, indent=2))

    send_notification(lsp, "initialized", {})

    # Open test document
    test_code = """divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(a / b)
}

main = () i32 {
    let result = divide(10.0, 2.0)
    return 0
}
"""

    uri = f"file://{os.getcwd()}/test_debug.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    time.sleep(0.5)

    # Test Hover
    print("\n\nHOVER REQUEST (line 0, char 2 - 'divide'):")
    send_request(lsp, "textDocument/hover", {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 2}
    }, 10)
    resp = read_response(lsp, 10)
    print(json.dumps(resp, indent=2))

    # Test Goto Definition
    print("\n\nGOTO DEFINITION REQUEST (line 5, char 19 - 'divide' call):")
    send_request(lsp, "textDocument/definition", {
        "textDocument": {"uri": uri},
        "position": {"line": 5, "character": 19}
    }, 11)
    resp = read_response(lsp, 11)
    print(json.dumps(resp, indent=2))

    # Test Document Symbols
    print("\n\nDOCUMENT SYMBOLS REQUEST:")
    send_request(lsp, "textDocument/documentSymbol", {
        "textDocument": {"uri": uri}
    }, 12)
    resp = read_response(lsp, 12)
    print(json.dumps(resp, indent=2))

finally:
    lsp.terminate()
    lsp.wait()
