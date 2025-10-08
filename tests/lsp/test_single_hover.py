#!/usr/bin/env python3
"""Test just hover to isolate the issue"""

import subprocess
import json
import time
import os

def main():
    print("Starting LSP server...")
    lsp = subprocess.Popen(
        ['/home/ubuntu/zenlang/target/debug/zen-lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    time.sleep(0.5)

    # Initialize
    workspace = f"file://{os.getcwd()}"

    def send_request(method, params, req_id):
        request = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params
        }
        msg = json.dumps(request)
        header = f"Content-Length: {len(msg)}\r\n\r\n"
        print(f"Sending {method}...")
        lsp.stdin.write((header + msg).encode())
        lsp.stdin.flush()

    def read_response():
        header = b""
        while b"\r\n\r\n" not in header:
            byte = lsp.stdout.read(1)
            if not byte:
                return None
            header += byte
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        return json.loads(body.decode())

    # Send initialize
    send_request("initialize", {
        "processId": os.getpid(),
        "rootUri": workspace,
        "capabilities": {}
    }, 1)
    init_resp = read_response()
    print(f"✅ Initialize response received")

    # Open document
    test_file = f"{workspace}/test_lsp_100.zen"
    test_content = """fn divide(a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        Result::Err("Division by zero")
    } else {
        Result::Ok(a / b)
    }
}

fn main() void {
    let x = 10
    let y = 3.14
    let msg = "Hello"
    let result = divide(x, y)
}"""

    send_request("textDocument/didOpen", {
        "textDocument": {
            "uri": test_file,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    }, 0)  # Notification, no response expected

    time.sleep(0.5)
    print(f"✅ Document opened")

    # Test hover
    send_request("textDocument/hover", {
        "textDocument": {"uri": test_file},
        "position": {"line": 0, "character": 5}  # Over "divide"
    }, 100)

    hover_resp = read_response()
    print(f"Hover response: {json.dumps(hover_resp, indent=2)}")

    # Cleanup
    lsp.terminate()
    lsp.wait()

if __name__ == '__main__':
    main()
