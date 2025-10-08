#!/usr/bin/env python3
"""Verify all LSP features work correctly"""

import json, subprocess, os, time

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

def read_response(lsp, expected_id=None):
    """Read responses until we find the one we want"""
    for _ in range(10):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)

        if expected_id and response.get("id") == expected_id:
            return response
        elif not expected_id and "method" not in response:
            return response
    return None

print("=" * 60)
print("ZEN LSP FEATURE VERIFICATION")
print("=" * 60)

test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

divide = (x: f64, y: f64) i32 {
    return 0
}

main = () i32 {
    value = 42
    result = add(10, 20)
    return value
}"""

with open("/tmp/lsp_test.log", "w") as stderr_file:
    lsp = subprocess.Popen(
        ["../../target/release/zen-lsp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=stderr_file
    )

    try:
        # 1. INITIALIZE
        print("\n1️⃣  Testing Initialize...")
        send_request(lsp, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {
                "textDocument": {
                    "hover": {"contentFormat": ["markdown", "plaintext"]},
                    "signatureHelp": {},
                    "rename": {},
                    "codeAction": {}
                }
            }
        }, 1)

        resp = read_response(lsp, 1)
        if resp and "result" in resp:
            print("  ✅ Initialize successful")
        else:
            print("  ❌ Initialize failed")

        send_notification(lsp, "initialized", {})

        # 2. OPEN DOCUMENT
        print("\n2️⃣  Opening test document...")
        uri = f"file://{os.getcwd()}/test_verify.zen"
        send_notification(lsp, "textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })
        time.sleep(0.3)
        print("  ✅ Document opened")

        # 3. HOVER
        print("\n3️⃣  Testing Hover...")
        send_request(lsp, "textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": 4, "character": 5}  # On "divide"
        }, 2)
        resp = read_response(lsp, 2)
        if resp and resp.get("result"):
            hover_text = resp["result"].get("contents", {})
            if isinstance(hover_text, dict):
                hover_text = hover_text.get("value", "")
            print(f"  ✅ Hover: {hover_text[:100]}...")
        else:
            print("  ❌ Hover failed")

        # 4. GOTO DEFINITION
        print("\n4️⃣  Testing Goto Definition...")
        send_request(lsp, "textDocument/definition", {
            "textDocument": {"uri": uri},
            "position": {"line": 10, "character": 15}  # On "add" in "add(10, 20)"
        }, 3)
        resp = read_response(lsp, 3)
        if resp and resp.get("result"):
            print(f"  ✅ Goto Definition: Found location")
        else:
            print("  ❌ Goto Definition failed")

        # 5. SIGNATURE HELP
        print("\n5️⃣  Testing Signature Help...")
        send_request(lsp, "textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": 10, "character": 18}  # Inside add(10, 20)
        }, 4)
        resp = read_response(lsp, 4)
        if resp and resp.get("result", {}).get("signatures"):
            sigs = resp["result"]["signatures"]
            print(f"  ✅ Signature Help: {sigs[0]['label']}")
        else:
            print("  ❌ Signature Help failed")

        # 6. INLAY HINTS
        print("\n6️⃣  Testing Inlay Hints...")
        send_request(lsp, "textDocument/inlayHint", {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 100, "character": 0}
            }
        }, 5)
        resp = read_response(lsp, 5)
        if resp and resp.get("result"):
            hints = resp["result"]
            print(f"  ✅ Inlay Hints: {len(hints)} hints found")
        else:
            print("  ❌ Inlay Hints failed")

        # 7. RENAME
        print("\n7️⃣  Testing Rename...")
        send_request(lsp, "textDocument/rename", {
            "textDocument": {"uri": uri},
            "position": {"line": 9, "character": 5},  # On "value"
            "newName": "myValue"
        }, 6)
        resp = read_response(lsp, 6)
        if resp and resp.get("result") and resp["result"].get("changes"):
            changes = resp["result"]["changes"]
            total_edits = sum(len(edits) for edits in changes.values())
            print(f"  ✅ Rename: {total_edits} edits across {len(changes)} files")
        else:
            print("  ❌ Rename failed")

        # 8. WORKSPACE SYMBOLS
        print("\n8️⃣  Testing Workspace Symbols...")
        send_request(lsp, "workspace/symbol", {"query": "add"}, 7)
        resp = read_response(lsp, 7)
        if resp and resp.get("result"):
            symbols = resp["result"]
            print(f"  ✅ Workspace Symbols: {len(symbols)} symbols found")
        else:
            print("  ❌ Workspace Symbols failed")

        # 9. DOCUMENT SYMBOLS
        print("\n9️⃣  Testing Document Symbols...")
        send_request(lsp, "textDocument/documentSymbol", {
            "textDocument": {"uri": uri}
        }, 8)
        resp = read_response(lsp, 8)
        if resp and resp.get("result"):
            symbols = resp["result"]
            print(f"  ✅ Document Symbols: {len(symbols)} symbols found")
        else:
            print("  ❌ Document Symbols failed")

        # 10. CODE ACTIONS
        print("\n🔟 Testing Code Actions...")
        send_request(lsp, "textDocument/codeAction", {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 12, "character": 0},
                "end": {"line": 12, "character": 20}
            },
            "context": {"diagnostics": []}
        }, 9)
        resp = read_response(lsp, 9)
        if resp and resp.get("result") is not None:
            actions = resp["result"]
            print(f"  ✅ Code Actions: {len(actions)} actions available")
        else:
            print("  ❌ Code Actions failed")

    finally:
        lsp.terminate()
        lsp.wait()

print("\n" + "=" * 60)
print("VERIFICATION COMPLETE")
print("=" * 60)
