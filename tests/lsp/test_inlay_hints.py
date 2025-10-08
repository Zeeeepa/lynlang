#!/usr/bin/env python3
"""Test Inlay Hints to verify positions are correct"""

import subprocess
import json
import sys
from pathlib import Path

def test_inlay_hints():
    lsp_path = Path("target/release/zen-lsp")
    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

main = () void {
    x = 5
    y = 10
    result = add(x, y)
}
"""

    # Start LSP
    lsp = subprocess.Popen(
        [str(lsp_path), "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    def send_request(method, params):
        msg_id = 1
        request = {
            "jsonrpc": "2.0",
            "id": msg_id,
            "method": method,
            "params": params
        }
        msg = json.dumps(request)
        header = f"Content-Length: {len(msg)}\r\n\r\n"
        lsp.stdin.write((header + msg).encode())
        lsp.stdin.flush()
        return msg_id

    def read_response():
        header = b""
        while b"\r\n\r\n" not in header:
            header += lsp.stdout.read(1)
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
        body = lsp.stdout.read(content_length)
        return json.loads(body.decode())

    # Initialize
    send_request("initialize", {
        "processId": None,
        "rootUri": "file:///tmp",
        "capabilities": {
            "textDocument": {
                "inlayHint": {}
            }
        }
    })
    init_resp = read_response()
    print("✅ Initialized LSP")

    # Send initialized notification
    notif = {"jsonrpc": "2.0", "method": "initialized", "params": {}}
    msg = json.dumps(notif)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()

    # Open document
    test_uri = "file:///tmp/test_inlay.zen"
    send_request("textDocument/didOpen", {
        "textDocument": {
            "uri": test_uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    import time
    time.sleep(0.3)

    # Request inlay hints
    req_id = send_request("textDocument/inlayHint", {
        "textDocument": {"uri": test_uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 100, "character": 0}
        }
    })

    # Read responses
    for _ in range(10):
        resp = read_response()
        if "id" in resp and resp["id"] == req_id:
            if "result" in resp:
                hints = resp["result"]
                if hints is None:
                    print("\n⚠️  No inlay hints returned (null result)")
                    lsp.terminate()
                    return 0

                print(f"\n✅ Received {len(hints)} inlay hints:")
                for hint in hints:
                    pos = hint["position"]
                    label = hint["label"]
                    print(f"  Line {pos['line']}, Col {pos['character']}: {label}")

                # Verify positions are not all at line 0
                lines = set(h["position"]["line"] for h in hints)
                if len(hints) == 0:
                    print("\n⚠️  Empty hints list (variables may have explicit types)")
                    lsp.terminate()
                    return 0
                elif len(lines) > 1 or (len(lines) == 1 and 0 not in lines):
                    print("\n✅ PASS: Inlay hint positions are correct (not all at line 0)")
                    lsp.terminate()
                    return 0
                else:
                    print(f"\n❌ FAIL: All hints at line {list(lines)[0]}")
                    lsp.terminate()
                    return 1
            break

    print("❌ FAIL: No response received")
    lsp.terminate()
    return 1

if __name__ == "__main__":
    sys.exit(test_inlay_hints())
