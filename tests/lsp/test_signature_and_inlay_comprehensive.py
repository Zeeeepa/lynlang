#!/usr/bin/env python3
"""Comprehensive test for Signature Help and Inlay Hints features"""

import subprocess
import json
import sys
import time
import select
from pathlib import Path

def send_request(lsp, msg_id_counter, method, params):
    msg_id_counter[0] += 1
    msg_id = msg_id_counter[0]
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

def send_notification(lsp, method, params):
    notif = {"jsonrpc": "2.0", "method": method, "params": params}
    msg = json.dumps(notif)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()

def read_response(lsp):
    header = b""
    while b"\r\n\r\n" not in header:
        chunk = lsp.stdout.read(1)
        if not chunk:
            return None
        header += chunk
    content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
    body = lsp.stdout.read(content_length)
    return json.loads(body.decode())

def drain_notifications(lsp):
    """Drain any pending notifications"""
    while select.select([lsp.stdout], [], [], 0.1)[0]:
        resp = read_response(lsp)
        if resp and resp.get("method") == "textDocument/publishDiagnostics":
            pass  # Ignore diagnostics

def test_signature_and_inlay():
    print("\n🧪 Testing Signature Help and Inlay Hints\n")

    lsp_path = Path("target/release/zen-lsp")
    if not lsp_path.exists():
        print("❌ LSP binary not found. Run: cargo build --release --bin zen-lsp")
        return 1

    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

divide = (a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        return Result.Err("Division by zero")
    }
    return Result.Ok(a / b)
}

main = () void {
    x = 42
    y = 3.14
    sum = add(10, 20)
    quotient = divide(100.0, 5.0)
}
"""

    # Start LSP
    lsp = subprocess.Popen(
        [str(lsp_path), "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    msg_id_counter = [0]

    try:
        # Initialize
        send_request(lsp, msg_id_counter, "initialize", {
            "processId": None,
            "rootUri": "file:///tmp",
            "capabilities": {
                "textDocument": {
                    "signatureHelp": {},
                    "inlayHint": {}
                }
            }
        })
        init_resp = read_response(lsp)
        if not init_resp or "result" not in init_resp:
            print("❌ Failed to initialize LSP")
            return 1
        print("✅ Initialized LSP")

        # Send initialized notification
        send_notification(lsp, "initialized", {})

        # Open document
        test_uri = "file:///tmp/test_comprehensive.zen"
        send_notification(lsp, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        time.sleep(0.3)
        drain_notifications(lsp)

        # Test 1: Signature Help for "add" function
        print("\n📝 Test 1: Signature Help for 'add(10, 20)'")
        req_id = send_request(lsp, msg_id_counter, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 15}  # Inside add(10, 20)
        })

        sig_help = None
        for _ in range(10):
            resp = read_response(lsp)
            if resp and resp.get("id") == req_id:
                sig_help = resp.get("result")
                break

        if sig_help and sig_help.get("signatures"):
            sig = sig_help["signatures"][0]
            print(f"  ✅ Signature: {sig['label']}")
            print(f"  ✅ Active parameter: {sig_help.get('activeParameter', 0)}")
            if sig.get("parameters"):
                print(f"  ✅ Parameters: {len(sig['parameters'])} params")
        else:
            print("  ❌ No signature help found")
            lsp.terminate()
            return 1

        # Test 2: Signature Help for "divide" function
        print("\n📝 Test 2: Signature Help for 'divide(100.0, 5.0)'")
        req_id = send_request(lsp, msg_id_counter, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 15, "character": 28}  # Inside divide(100.0, 5.0)
        })

        sig_help = None
        for _ in range(10):
            resp = read_response(lsp)
            if resp and resp.get("id") == req_id:
                sig_help = resp.get("result")
                break

        if sig_help and sig_help.get("signatures"):
            sig = sig_help["signatures"][0]
            print(f"  ✅ Signature: {sig['label']}")
            print(f"  ✅ Active parameter: {sig_help.get('activeParameter', 0)}")
        else:
            print("  ❌ No signature help found")
            lsp.terminate()
            return 1

        # Test 3: Inlay Hints
        print("\n📝 Test 3: Inlay Hints for type annotations")
        req_id = send_request(lsp, msg_id_counter, "textDocument/inlayHint", {
            "textDocument": {"uri": test_uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 100, "character": 0}
            }
        })

        hints = None
        for _ in range(10):
            resp = read_response(lsp)
            if resp and resp.get("id") == req_id:
                hints = resp.get("result")
                break

        if hints is not None:
            print(f"  ✅ Received {len(hints)} inlay hints")
            for hint in hints[:5]:  # Show first 5
                pos = hint["position"]
                label = hint["label"]
                print(f"    Line {pos['line']}, Col {pos['character']}: {label}")

            if len(hints) > 5:
                print(f"    ... and {len(hints) - 5} more hints")
        else:
            print("  ❌ No inlay hints found")
            lsp.terminate()
            return 1

        print("\n🎉 All tests PASSED!")
        lsp.terminate()
        return 0

    except Exception as e:
        print(f"\n❌ Test failed with exception: {e}")
        import traceback
        traceback.print_exc()
        lsp.terminate()
        return 1

if __name__ == "__main__":
    sys.exit(test_signature_and_inlay())
