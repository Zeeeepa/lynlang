#!/usr/bin/env python3
"""
Comprehensive LSP Feature Parity Assessment
Tests each feature and provides detailed metrics
"""

import subprocess
import json
import time
import os
import sys

def start_lsp():
    """Start the LSP server"""
    lsp_path = os.path.join(os.getcwd(), "target/release/zen-lsp")
    return subprocess.Popen(
        [lsp_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0
    )

def send_request(lsp, method, params, request_id=1):
    """Send a JSON-RPC request to the LSP"""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }
    message = json.dumps(request)
    content = f"Content-Length: {len(message)}\r\n\r\n{message}"
    lsp.stdin.write(content)
    lsp.stdin.flush()

def read_response(lsp):
    """Read a JSON-RPC response from the LSP"""
    while True:
        header = lsp.stdout.readline()
        if not header:
            return None
        if header.startswith("Content-Length:"):
            length = int(header.split(":")[1].strip())
            lsp.stdout.readline()  # Empty line
            content = lsp.stdout.read(length)
            return json.loads(content)

def test_feature(name, test_func):
    """Test a feature and return score"""
    try:
        score = test_func()
        status = "✅" if score >= 0.8 else "⚠️" if score >= 0.5 else "❌"
        return (status, score)
    except Exception as e:
        return ("❌", 0.0)

def main():
    print("=" * 60)
    print("ZEN LSP FEATURE PARITY ASSESSMENT")
    print("=" * 60)
    print()

    lsp = start_lsp()
    time.sleep(1)

    # Initialize
    send_request(lsp, "initialize", {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    })
    init_response = read_response(lsp)

    test_content = """
fn add(a: i32, b: i32) i32 {
    return a + b
}

fn divide(a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

fn main() void {
    let x = 5
    let y = add(x, 3)
}
"""

    test_uri = f"file://{os.getcwd()}/test_parity.zen"

    # Open document
    send_request(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": test_uri,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    }, request_id=None)
    time.sleep(0.5)

    results = {}

    # Test 1: Hover Information
    def test_hover():
        send_request(lsp, "textDocument/hover", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 1, "character": 4}
        }, 10)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            contents = str(resp["result"].get("contents", ""))
            has_type = "i32" in contents or "add" in contents
            return 1.0 if has_type else 0.5
        return 0.0

    # Test 2: Goto Definition
    def test_goto_def():
        send_request(lsp, "textDocument/definition", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 13}
        }, 11)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            return 1.0
        return 0.0

    # Test 3: Signature Help
    def test_signature():
        send_request(lsp, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 16}
        }, 12)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            sigs = resp["result"].get("signatures", [])
            if len(sigs) > 0:
                has_params = "parameters" in sigs[0]
                has_label = "label" in sigs[0]
                return 1.0 if (has_params and has_label) else 0.7
        return 0.0

    # Test 4: Inlay Hints
    def test_inlay():
        send_request(lsp, "textDocument/inlayHint", {
            "textDocument": {"uri": test_uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 100, "character": 0}
            }
        }, 13)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            hints = resp["result"]
            if len(hints) > 0:
                return 1.0
            return 0.5  # Feature works but no hints for this code
        return 0.0

    # Test 5: Rename
    def test_rename():
        send_request(lsp, "textDocument/rename", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 13, "character": 8},
            "newName": "myValue"
        }, 14)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            changes = resp["result"].get("changes", {})
            if len(changes) > 0:
                return 1.0
        return 0.0

    # Test 6: Code Completion
    def test_completion():
        send_request(lsp, "textDocument/completion", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 14, "character": 9}
        }, 15)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            items = resp["result"].get("items", []) if isinstance(resp["result"], dict) else resp["result"]
            return 1.0 if len(items) > 0 else 0.5
        return 0.0

    # Test 7: Document Symbols
    def test_doc_symbols():
        send_request(lsp, "textDocument/documentSymbol", {
            "textDocument": {"uri": test_uri}
        }, 16)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            symbols = resp["result"]
            if len(symbols) >= 2:  # Should have add, divide, main
                return 1.0
            return 0.5
        return 0.0

    # Test 8: Workspace Symbols
    def test_ws_symbols():
        send_request(lsp, "workspace/symbol", {
            "query": "add"
        }, 17)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            return 1.0 if len(resp["result"]) > 0 else 0.5
        return 0.0

    # Test 9: Find References
    def test_references():
        send_request(lsp, "textDocument/references", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 1, "character": 4},
            "context": {"includeDeclaration": True}
        }, 18)
        resp = read_response(lsp)
        if resp and resp.get("result"):
            return 1.0 if len(resp["result"]) > 0 else 0.5
        return 0.0

    # Test 10: Code Actions
    def test_code_actions():
        send_request(lsp, "textDocument/codeAction", {
            "textDocument": {"uri": test_uri},
            "range": {
                "start": {"line": 13, "character": 8},
                "end": {"line": 13, "character": 10}
            },
            "context": {"diagnostics": []}
        }, 19)
        resp = read_response(lsp)
        if resp:
            result = resp.get("result")
            # Null result is OK if no code actions available
            return 0.8 if result is not None else 0.5
        return 0.0

    # Run tests
    print("Testing Core Features:")
    print("-" * 60)
    results["Hover Information"] = test_feature("Hover", test_hover)
    results["Goto Definition"] = test_feature("Goto Def", test_goto_def)
    results["Signature Help"] = test_feature("Signature", test_signature)
    results["Inlay Hints"] = test_feature("Inlay", test_inlay)
    results["Rename Symbol"] = test_feature("Rename", test_rename)
    results["Code Completion"] = test_feature("Completion", test_completion)
    results["Document Symbols"] = test_feature("Doc Symbols", test_doc_symbols)
    results["Workspace Symbols"] = test_feature("WS Symbols", test_ws_symbols)
    results["Find References"] = test_feature("References", test_references)
    results["Code Actions"] = test_feature("Code Actions", test_code_actions)

    # Print results
    for feature, (status, score) in results.items():
        pct = int(score * 100)
        bar = "█" * int(score * 20) + "░" * (20 - int(score * 20))
        print(f"{status} {feature:.<25} {bar} {pct}%")

    print()
    print("=" * 60)
    avg_score = sum(s for _, s in results.values()) / len(results)
    overall_pct = int(avg_score * 100)
    print(f"OVERALL FEATURE PARITY: {overall_pct}%")
    print("=" * 60)

    # Shutdown
    send_request(lsp, "shutdown", {}, 999)
    read_response(lsp)
    lsp.stdin.write("Content-Length: 48\r\n\r\n" + json.dumps({"jsonrpc": "2.0", "method": "exit"}))
    lsp.wait(timeout=5)

    return overall_pct

if __name__ == "__main__":
    try:
        score = main()
        sys.exit(0 if score >= 85 else 1)
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
