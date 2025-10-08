#!/usr/bin/env python3
"""Comprehensive test for all core LSP features"""

import json
import subprocess
import os
import time

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

def read_response(proc, req_id, timeout_count=10):
    """Read response with specific request ID"""
    for _ in range(timeout_count):
        line = proc.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        proc.stdout.readline()
        content = proc.stdout.read(length).decode()
        response = json.loads(content)
        
        if response.get("id") == req_id:
            return response
        # Skip notifications
    return None

print("=" * 60)
print("ZEN LSP COMPREHENSIVE FEATURE TEST")
print("=" * 60)

lsp = subprocess.Popen(
    ["./target/debug/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

test_results = []

try:
    # Initialize
    print("\n1. Testing Initialization...")
    send_request(lsp, "initialize", {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    }, 1)
    init_response = read_response(lsp, 1)
    
    if init_response and init_response.get("result"):
        capabilities = init_response["result"]["capabilities"]
        print(f"   ✅ LSP initialized")
        print(f"   - Hover: {bool(capabilities.get('hoverProvider'))}")
        print(f"   - Goto Definition: {bool(capabilities.get('definitionProvider'))}")
        print(f"   - Rename: {bool(capabilities.get('renameProvider'))}")
        print(f"   - Signature Help: {bool(capabilities.get('signatureHelpProvider'))}")
        print(f"   - Inlay Hints: {bool(capabilities.get('inlayHintProvider'))}")
        test_results.append(("Initialization", True))
    else:
        print("   ❌ Initialization failed")
        test_results.append(("Initialization", False))

    send_notification(lsp, "initialized", {})
    time.sleep(0.1)

    # Open test document
    test_code = """divide = (a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

main = () i32 {
    value = 42
    result = divide(10.0, 2.0)
    match result {
        Ok(val) => print("Success")
        Err(msg) => print(msg)
    }
    return 0
}"""

    uri = f"file://{os.getcwd()}/test_comprehensive.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })
    time.sleep(0.3)

    # Test 2: Hover
    print("\n2. Testing Hover...")
    send_request(lsp, "textDocument/hover", {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 0}
    }, 2)
    hover_response = read_response(lsp, 2)
    
    if hover_response and hover_response.get("result"):
        hover_content = hover_response["result"].get("contents")
        print(f"   ✅ Hover works")
        if hover_content:
            content_str = hover_content.get("value", str(hover_content))
            print(f"   - Content: {content_str[:60]}...")
        test_results.append(("Hover", True))
    else:
        print("   ❌ Hover failed")
        test_results.append(("Hover", False))

    # Test 3: Goto Definition
    print("\n3. Testing Goto Definition...")
    send_request(lsp, "textDocument/definition", {
        "textDocument": {"uri": uri},
        "position": {"line": 9, "character": 13}  # "divide" in main
    }, 3)
    goto_response = read_response(lsp, 3)
    
    if goto_response and goto_response.get("result"):
        result = goto_response["result"]
        # Result can be a Location or Location[]
        if isinstance(result, list):
            if len(result) > 0:
                print(f"   ✅ Goto Definition works")
                print(f"   - Found {len(result)} location(s)")
                test_results.append(("Goto Definition", True))
            else:
                print(f"   ⚠️  Goto Definition returned empty result")
                test_results.append(("Goto Definition", False))
        elif isinstance(result, dict) and "uri" in result:
            print(f"   ✅ Goto Definition works")
            print(f"   - Location: line {result.get('range', {}).get('start', {}).get('line')}")
            test_results.append(("Goto Definition", True))
        else:
            print(f"   ⚠️  Unexpected goto definition result")
            test_results.append(("Goto Definition", False))
    else:
        print("   ❌ Goto Definition failed")
        test_results.append(("Goto Definition", False))

    # Test 4: Signature Help
    print("\n4. Testing Signature Help...")
    send_request(lsp, "textDocument/signatureHelp", {
        "textDocument": {"uri": uri},
        "position": {"line": 9, "character": 20}  # Inside divide() call
    }, 4)
    sig_response = read_response(lsp, 4)
    
    if sig_response and sig_response.get("result"):
        signatures = sig_response["result"].get("signatures", [])
        if signatures:
            print(f"   ✅ Signature Help works")
            print(f"   - Signature: {signatures[0].get('label')}")
            test_results.append(("Signature Help", True))
        else:
            print("   ⚠️  Signature Help returned empty")
            test_results.append(("Signature Help", False))
    else:
        print("   ❌ Signature Help failed")
        test_results.append(("Signature Help", False))

    # Test 5: Inlay Hints
    print("\n5. Testing Inlay Hints...")
    send_request(lsp, "textDocument/inlayHint", {
        "textDocument": {"uri": uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 20, "character": 0}
        }
    }, 5)
    inlay_response = read_response(lsp, 5)
    
    if inlay_response and inlay_response.get("result") is not None:
        hints = inlay_response["result"]
        print(f"   ✅ Inlay Hints works")
        print(f"   - Found {len(hints)} hint(s)")
        for hint in hints[:3]:  # Show first 3
            pos = hint.get("position", {})
            label = hint.get("label")
            print(f"     • Line {pos.get('line')}, Col {pos.get('character')}: {label}")
        test_results.append(("Inlay Hints", True))
    else:
        print("   ❌ Inlay Hints failed")
        test_results.append(("Inlay Hints", False))

    # Test 6: Rename
    print("\n6. Testing Rename...")
    send_request(lsp, "textDocument/rename", {
        "textDocument": {"uri": uri},
        "position": {"line": 8, "character": 4},  # "value" variable
        "newName": "myValue"
    }, 6)
    rename_response = read_response(lsp, 6)
    
    if rename_response and rename_response.get("result"):
        changes = rename_response["result"].get("changes", {})
        total_edits = sum(len(edits) for edits in changes.values())
        if total_edits > 0:
            print(f"   ✅ Rename works")
            print(f"   - {total_edits} edit(s) in {len(changes)} file(s)")
            test_results.append(("Rename", True))
        else:
            print("   ⚠️  Rename returned no edits")
            test_results.append(("Rename", False))
    else:
        print("   ❌ Rename failed")
        test_results.append(("Rename", False))

    # Test 7: Workspace Symbols
    print("\n7. Testing Workspace Symbols...")
    send_request(lsp, "workspace/symbol", {
        "query": "divide"
    }, 7)
    symbol_response = read_response(lsp, 7)
    
    if symbol_response and symbol_response.get("result"):
        symbols = symbol_response["result"]
        print(f"   ✅ Workspace Symbols works")
        print(f"   - Found {len(symbols)} symbol(s)")
        test_results.append(("Workspace Symbols", True))
    else:
        print("   ❌ Workspace Symbols failed")
        test_results.append(("Workspace Symbols", False))

    # Test 8: Document Symbols
    print("\n8. Testing Document Symbols...")
    send_request(lsp, "textDocument/documentSymbol", {
        "textDocument": {"uri": uri}
    }, 8)
    doc_sym_response = read_response(lsp, 8)
    
    if doc_sym_response and doc_sym_response.get("result") is not None:
        doc_symbols = doc_sym_response["result"]
        print(f"   ✅ Document Symbols works")
        print(f"   - Found {len(doc_symbols)} symbol(s)")
        if doc_symbols:
            for sym in doc_symbols[:2]:
                print(f"     • {sym.get('name')}: {sym.get('detail', 'N/A')[:40]}")
        test_results.append(("Document Symbols", True))
    else:
        print("   ❌ Document Symbols failed")
        test_results.append(("Document Symbols", False))

finally:
    lsp.terminate()
    lsp.wait()

# Print summary
print("\n" + "=" * 60)
print("TEST SUMMARY")
print("=" * 60)

passed = sum(1 for _, result in test_results if result)
total = len(test_results)
percentage = (passed / total * 100) if total > 0 else 0

for feature, result in test_results:
    status = "✅ PASS" if result else "❌ FAIL"
    print(f"{status} - {feature}")

print("\n" + "=" * 60)
print(f"TOTAL: {passed}/{total} tests passed ({percentage:.1f}%)")
print("=" * 60)

if percentage >= 90:
    print("\n🎉 EXCELLENT! LSP is production ready!")
elif percentage >= 75:
    print("\n✅ GOOD! LSP is mostly functional")
else:
    print("\n⚠️  NEEDS WORK - Some features not working")
