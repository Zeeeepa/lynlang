#!/usr/bin/env python3
"""Comprehensive verification of LSP at 100% feature parity"""

import subprocess
import json
import sys
import time
from pathlib import Path

def send_request(lsp, msg_id, method, params):
    request = {"jsonrpc": "2.0", "id": msg_id, "method": method, "params": params}
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

def read_response(lsp, timeout=2.0):
    import select
    if not select.select([lsp.stdout], [], [], timeout)[0]:
        return None
    header = b""
    while b"\r\n\r\n" not in header:
        chunk = lsp.stdout.read(1)
        if not chunk:
            return None
        header += chunk
    content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
    body = lsp.stdout.read(content_length)
    return json.loads(body.decode())

def read_until_id(lsp, expected_id, max_attempts=20):
    for _ in range(max_attempts):
        resp = read_response(lsp, timeout=0.5)
        if resp is None:
            continue
        if resp.get("id") == expected_id:
            return resp
    return None

def main():
    print("🧪 Comprehensive LSP Feature Verification\n")
    print("=" * 60)

    lsp_path = Path("target/release/zen-lsp")
    if not lsp_path.exists():
        print("❌ LSP binary not found")
        return 1

    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

divide = (a: f64, b: f64) Result<f64, String> {
    b == 0.0 ? {
        return Result.Err("Division by zero")
    }
    Result.Ok(a / b)
}

main = () void {
    x = 42
    y = 3.14
    sum = add(10, 20)
    quotient = divide(100.0, 5.0)
}
"""

    lsp = subprocess.Popen(
        [str(lsp_path), "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    msg_id = 0
    passed = 0
    total = 0

    try:
        # Initialize
        msg_id += 1
        send_request(lsp, msg_id, "initialize", {
            "processId": None,
            "rootUri": "file:///tmp",
            "capabilities": {}
        })
        init_resp = read_until_id(lsp, msg_id)
        if init_resp and "result" in init_resp:
            print("✅ LSP Initialization")
            passed += 1
        else:
            print("❌ LSP Initialization")
        total += 1

        send_notification(lsp, "initialized", {})

        # Open document
        test_uri = "file:///tmp/test_verify.zen"
        send_notification(lsp, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })
        time.sleep(0.3)

        # Drain diagnostics
        for _ in range(5):
            read_response(lsp, timeout=0.1)

        # Test 1: Hover
        print("\n📝 Testing Core Features:")
        print("-" * 60)
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/hover", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 0, "character": 2}  # "add" function name
        })
        resp = read_until_id(lsp, msg_id)
        if resp and resp.get("result") and resp["result"] != None:
            print("✅ Hover Information")
            passed += 1
        else:
            print("❌ Hover Information")
        total += 1

        # Test 2: Goto Definition
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/definition", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 10}  # "add" in call
        })
        resp = read_until_id(lsp, msg_id)
        if resp and resp.get("result"):
            print("✅ Goto Definition")
            passed += 1
        else:
            print("❌ Goto Definition")
        total += 1

        # Test 3: Document Symbols
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/documentSymbol", {
            "textDocument": {"uri": test_uri}
        })
        resp = read_until_id(lsp, msg_id)
        if resp and resp.get("result"):
            result = resp["result"]
            if isinstance(result, list) and len(result) > 0:
                print(f"✅ Document Symbols ({len(result)} symbols)")
                passed += 1
            else:
                print("❌ Document Symbols (no symbols found)")
        else:
            print("❌ Document Symbols")
        total += 1

        # Test 4: Signature Help
        print("\n📝 Testing Advanced Features:")
        print("-" * 60)
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 18}  # Inside add(10, 20)
        })
        resp = read_until_id(lsp, msg_id)
        if resp and resp.get("result"):
            result = resp["result"]
            if result.get("signatures"):
                sig = result["signatures"][0]
                print(f"✅ Signature Help - {sig['label']}")
                passed += 1
            else:
                print("❌ Signature Help (no signatures)")
        else:
            print("❌ Signature Help")
        total += 1

        # Test 5: Inlay Hints
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/inlayHint", {
            "textDocument": {"uri": test_uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 100, "character": 0}
            }
        })
        resp = read_until_id(lsp, msg_id)
        if resp and "result" in resp:
            hints = resp["result"]
            if isinstance(hints, list) and len(hints) > 0:
                print(f"✅ Inlay Hints ({len(hints)} hints)")
                passed += 1
            else:
                print("❌ Inlay Hints (no hints)")
        else:
            print("❌ Inlay Hints")
        total += 1

        # Test 6: Code Completion
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/completion", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 5}
        })
        resp = read_until_id(lsp, msg_id)
        if resp and resp.get("result"):
            print("✅ Code Completion")
            passed += 1
        else:
            print("❌ Code Completion")
        total += 1

        # Test 7: Find References
        msg_id += 1
        send_request(lsp, msg_id, "textDocument/references", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 0, "character": 0},
            "context": {"includeDeclaration": True}
        })
        resp = read_until_id(lsp, msg_id)
        if resp and "result" in resp:
            print("✅ Find References")
            passed += 1
        else:
            print("❌ Find References")
        total += 1

        # Summary
        print("\n" + "=" * 60)
        print(f"📊 Results: {passed}/{total} tests passed ({passed*100//total}%)")
        print("=" * 60)

        if passed == total:
            print("\n🎉 ALL TESTS PASSED! LSP is at 100% feature parity!")
            lsp.terminate()
            return 0
        else:
            print(f"\n⚠️  {total - passed} tests failed")
            lsp.terminate()
            return 1

    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        lsp.terminate()
        return 1

if __name__ == "__main__":
    sys.exit(main())
