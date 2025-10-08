#!/usr/bin/env python3
"""Verify the 4 features that were listed as missing for 100% parity"""

import subprocess
import json
import time
import sys

def start_lsp():
    proc = subprocess.Popen(
        ['cargo', 'run', '--bin', 'zen', '--', 'lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd='../..'
    )
    time.sleep(1)
    return proc

def send_request(proc, method, params):
    request_id = 1
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }
    message = json.dumps(request)
    content_length = len(message)
    
    proc.stdin.write(f"Content-Length: {content_length}\r\n\r\n{message}".encode())
    proc.stdin.flush()
    
    # Read response
    while True:
        line = proc.stdout.readline().decode()
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            proc.stdout.readline()  # Skip blank line
            response = proc.stdout.read(length).decode()
            return json.loads(response)

def test_rename_symbol(proc, uri):
    """Test 1: Rename Symbol (was 0% done)"""
    params = {
        "textDocument": {"uri": uri},
        "position": {"line": 1, "character": 8},
        "newName": "newVarName"
    }
    
    result = send_request(proc, "textDocument/rename", params)
    if result and 'result' in result and result['result']:
        edits = result['result'].get('changes', {})
        if edits:
            print("✅ Test 1 PASSED: Rename Symbol - Working!")
            return True
    print("❌ Test 1 FAILED: Rename Symbol - Not working")
    return False

def test_signature_help(proc, uri):
    """Test 2: Signature Help (was 10% - stubbed)"""
    params = {
        "textDocument": {"uri": uri},
        "position": {"line": 5, "character": 15}
    }
    
    result = send_request(proc, "textDocument/signatureHelp", params)
    if result and 'result' in result and result['result']:
        sigs = result['result'].get('signatures', [])
        if sigs:
            print(f"✅ Test 2 PASSED: Signature Help - Working! ({len(sigs)} signature(s))")
            return True
    print("❌ Test 2 FAILED: Signature Help - Not working")
    return False

def test_inlay_hints(proc, uri):
    """Test 3: Inlay Hints (was 10% - stubbed)"""
    params = {
        "textDocument": {"uri": uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 10, "character": 0}
        }
    }
    
    result = send_request(proc, "textDocument/inlayHint", params)
    if result and 'result' in result and result['result']:
        hints = result['result']
        if len(hints) > 0:
            print(f"✅ Test 3 PASSED: Inlay Hints - Working! ({len(hints)} hint(s))")
            return True
    print("❌ Test 3 FAILED: Inlay Hints - Not working")
    return False

def test_find_references(proc, uri):
    """Test 4: AST-based Find References (was 70% - text-based)"""
    params = {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 5},
        "context": {"includeDeclaration": True}
    }
    
    result = send_request(proc, "textDocument/references", params)
    if result and 'result' in result:
        refs = result['result']
        if refs and len(refs) > 0:
            print(f"✅ Test 4 PASSED: Find References - Working! ({len(refs)} reference(s))")
            return True
    print("❌ Test 4 FAILED: Find References - Not working")
    return False

print("=" * 60)
print("VERIFYING 4 FEATURES NEEDED FOR 100% PARITY")
print("=" * 60)
print()

# Create test file
test_code = """compute = (x: i32, y: i32) i32 {
    let sum = x + y;
    return sum;
}

main = () i32 {
    let result = compute(10, 20);
    return result;
}
"""

with open('verify_test.zen', 'w') as f:
    f.write(test_code)

uri = "file://" + subprocess.check_output(['pwd']).decode().strip() + "/verify_test.zen"

proc = start_lsp()

# Initialize
send_request(proc, "initialize", {
    "capabilities": {},
    "rootUri": "file://" + subprocess.check_output(['pwd']).decode().strip()
})
send_request(proc, "initialized", {})

# Open document
send_request(proc, "textDocument/didOpen", {
    "textDocument": {
        "uri": uri,
        "languageId": "zen",
        "version": 1,
        "text": test_code
    }
})

time.sleep(0.5)

# Run tests
results = []
results.append(test_rename_symbol(proc, uri))
results.append(test_signature_help(proc, uri))
results.append(test_inlay_hints(proc, uri))
results.append(test_find_references(proc, uri))

proc.terminate()

print()
print("=" * 60)
print(f"RESULTS: {sum(results)}/4 features working")
print("=" * 60)

if sum(results) == 4:
    print("🎉 100% FEATURE PARITY CONFIRMED!")
    sys.exit(0)
else:
    print(f"⚠️  {4-sum(results)} feature(s) still need work")
    sys.exit(1)
