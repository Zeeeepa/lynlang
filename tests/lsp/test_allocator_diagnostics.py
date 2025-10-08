#!/usr/bin/env python3
"""Test LSP Allocator Diagnostics and Quick Fixes"""

import subprocess
import json
import time
from pathlib import Path

def send_request(lsp, msg_id, method, params):
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

def read_response(lsp):
    header = b""
    while b"\r\n\r\n" not in header:
        header += lsp.stdout.read(1)
    content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
    body = lsp.stdout.read(content_length)
    return json.loads(body.decode())

def test_allocator_diagnostics():
    # Test code with allocator issues
    test_code = """{ HashMap, DynVec, get_default_allocator } = @std

main = () void {
    // Missing allocator - should trigger diagnostic
    map = HashMap.new<StaticString, i32>()

    // Missing allocator - should trigger diagnostic
    vec = DynVec.new<i32>()

    // Correct usage with allocator
    allocator = get_default_allocator()
    good_map = HashMap.new<StaticString, i32>(allocator)

    return 0
}
"""

    test_file = Path("/tmp/test_allocator.zen")
    test_file.write_text(test_code)

    lsp_path = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    lsp = subprocess.Popen(
        [str(lsp_path), "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    try:
        # Initialize
        send_request(lsp, 1, "initialize", {
            "processId": None,
            "rootUri": "file:///tmp",
            "capabilities": {}
        })
        read_response(lsp)

        # Initialized notification
        notif = {"jsonrpc": "2.0", "method": "initialized", "params": {}}
        msg = json.dumps(notif)
        lsp.stdin.write(f"Content-Length: {len(msg)}\r\n\r\n{msg}".encode())
        lsp.stdin.flush()

        # Open document
        send_request(lsp, 2, "textDocument/didOpen", {
            "textDocument": {
                "uri": str(test_file.as_uri()),
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        # Wait for diagnostics
        time.sleep(0.5)

        # Read any diagnostic notifications
        diagnostics_found = []
        try:
            lsp.stdout.peek()  # Check if data available
            while True:
                response = read_response(lsp)
                if response.get("method") == "textDocument/publishDiagnostics":
                    diagnostics_found.extend(response.get("params", {}).get("diagnostics", []))
                    break
        except:
            pass

        print("=" * 70)
        print("ALLOCATOR DIAGNOSTICS TEST")
        print("=" * 70)
        print()

        if diagnostics_found:
            print(f"✅ Found {len(diagnostics_found)} allocator diagnostic(s):")
            for diag in diagnostics_found:
                print(f"  - Line {diag['range']['start']['line']}: {diag['message']}")
                if 'code' in diag:
                    print(f"    Code: {diag['code']}")
        else:
            print("⚠️  No allocator diagnostics found")

        # Test code action (quick fix)
        if diagnostics_found:
            print("\nTesting Code Action (Quick Fix)...")
            send_request(lsp, 3, "textDocument/codeAction", {
                "textDocument": {"uri": str(test_file.as_uri())},
                "range": diagnostics_found[0]["range"],
                "context": {
                    "diagnostics": [diagnostics_found[0]]
                }
            })

            action_response = read_response(lsp)
            if action_response.get("result"):
                print(f"✅ Quick fix available: {action_response['result'][0]['title']}")
            else:
                print("❌ No quick fix provided")

        print("\n" + "=" * 70)

    finally:
        lsp.terminate()
        lsp.wait()
        test_file.unlink(missing_ok=True)

if __name__ == "__main__":
    test_allocator_diagnostics()
