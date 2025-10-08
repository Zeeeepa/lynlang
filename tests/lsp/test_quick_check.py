#!/usr/bin/env python3
"""Quick test to check hover functionality"""

import subprocess
import json
import time
import os

def main():
    lsp = subprocess.Popen(
        ['/home/ubuntu/zenlang/target/debug/zen-lsp'],
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

    def read_response():
        header = b""
        while b"\r\n\r\n" not in header:
            header += lsp.stdout.read(1)
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        return json.loads(body.decode())

    workspace = f"file://{os.getcwd()}"

    # Initialize
    send_request("initialize", {
        "processId": os.getpid(),
        "rootUri": workspace,
        "capabilities": {}
    }, 1)
    init_resp = read_response()
    print("✅ Initialized")

    # Send initialized notification
    send_notification("initialized", {})
    time.sleep(0.1)

    # Open document
    test_file = f"{workspace}/test_lsp_100.zen"
    test_content = """{ Result } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(a / b)
}

main = () i32 {
    result = divide(10.0, 5.0)
    return 0
}"""

    send_notification("textDocument/didOpen", {
        "textDocument": {
            "uri": test_file,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    })
    time.sleep(0.5)
    print("✅ Document opened")

    # Test hover
    send_request("textDocument/hover", {
        "textDocument": {"uri": test_file},
        "position": {"line": 2, "character": 0}  # Over "divide"
    }, 100)
    hover_resp = read_response()
    print(f"Hover response: {json.dumps(hover_resp, indent=2)}")

    lsp.terminate()

if __name__ == '__main__':
    main()
