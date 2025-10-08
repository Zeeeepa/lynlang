#!/usr/bin/env python3
"""Simple test to check if LSP starts up correctly"""

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

    time.sleep(1.0)

    # Check if process is still running
    if lsp.poll() is not None:
        print("❌ LSP server crashed!")
        stderr = lsp.stderr.read()
        print(f"STDERR: {stderr.decode()}")
        return 1

    print("✅ LSP server started successfully")

    # Try to send initialize request
    workspace = f"file://{os.getcwd()}"
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": os.getpid(),
            "rootUri": workspace,
            "capabilities": {}
        }
    }
    msg = json.dumps(request)
    header = f"Content-Length: {len(msg)}\r\n\r\n"

    print(f"Sending initialize request...")
    try:
        lsp.stdin.write((header + msg).encode())
        lsp.stdin.flush()
        print("✅ Request sent")
    except BrokenPipeError:
        print("❌ Broken pipe when sending request")
        stderr = lsp.stderr.read()
        print(f"STDERR: {stderr.decode()}")
        return 1

    # Try to read response
    print("Reading response...")
    try:
        header_bytes = b""
        while b"\r\n\r\n" not in header_bytes:
            byte = lsp.stdout.read(1)
            if not byte:
                print("❌ No data from LSP server")
                stderr = lsp.stderr.read()
                print(f"STDERR: {stderr.decode()}")
                return 1
            header_bytes += byte

        content_length = int(header_bytes.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        response = json.loads(body.decode())

        print(f"✅ Got response: {json.dumps(response, indent=2)}")

        # Clean shutdown
        lsp.terminate()
        lsp.wait()
        return 0

    except Exception as e:
        print(f"❌ Error reading response: {e}")
        stderr = lsp.stderr.read()
        print(f"STDERR: {stderr.decode()}")
        return 1

if __name__ == '__main__':
    exit(main())
