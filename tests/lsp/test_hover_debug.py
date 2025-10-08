#!/usr/bin/env python3
"""Debug hover crash"""

import subprocess
import json
import time
import os
import select

def main():
    print("Starting LSP server...")
    lsp = subprocess.Popen(
        ['/home/ubuntu/zenlang/target/debug/zen-lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    def check_stderr():
        """Non-blocking check of stderr"""
        if select.select([lsp.stderr], [], [], 0)[0]:
            err = lsp.stderr.read()
            if err:
                print(f"STDERR: {err.decode()}")

    time.sleep(0.5)
    check_stderr()

    # Initialize
    workspace = f"file://{os.getcwd()}"

    def send_request(method, params, req_id):
        if lsp.poll() is not None:
            print(f"❌ LSP died! Exit code: {lsp.returncode}")
            check_stderr()
            return False

        request = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params
        }
        msg = json.dumps(request)
        header = f"Content-Length: {len(msg)}\r\n\r\n"
        print(f"Sending {method}...")
        try:
            lsp.stdin.write((header + msg).encode())
            lsp.stdin.flush()
            return True
        except BrokenPipeError:
            print(f"❌ Broken pipe sending {method}")
            check_stderr()
            return False

    def read_response():
        if lsp.poll() is not None:
            print(f"❌ LSP died! Exit code: {lsp.returncode}")
            check_stderr()
            return None

        header = b""
        while b"\r\n\r\n" not in header:
            byte = lsp.stdout.read(1)
            if not byte:
                print("❌ No response from LSP")
                check_stderr()
                return None
            header += byte
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        return json.loads(body.decode())

    # Send initialize
    if not send_request("initialize", {
        "processId": os.getpid(),
        "rootUri": workspace,
        "capabilities": {}
    }, 1):
        return 1

    init_resp = read_response()
    if not init_resp:
        return 1
    print(f"✅ Initialize OK")

    # Open document
    test_file = f"{workspace}/test_lsp_100.zen"
    test_content = """fn divide(a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        Result::Err("Division by zero")
    } else {
        Result::Ok(a / b)
    }
}"""

    if not send_request("textDocument/didOpen", {
        "textDocument": {
            "uri": test_file,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    }, 0):
        return 1

    time.sleep(0.5)
    check_stderr()
    print(f"✅ Document opened")

    # Test hover
    print("About to send hover request...")
    if not send_request("textDocument/hover", {
        "textDocument": {"uri": test_file},
        "position": {"line": 0, "character": 5}  # Over "divide"
    }, 100):
        print("Failed to send hover")
        return 1

    time.sleep(0.5)
    check_stderr()

    hover_resp = read_response()
    if hover_resp:
        print(f"Hover response: {json.dumps(hover_resp, indent=2)}")
    else:
        print("No hover response received")

    # Cleanup
    lsp.terminate()
    lsp.wait()

if __name__ == '__main__':
    exit(main())
