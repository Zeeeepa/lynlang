#!/usr/bin/env python3
"""Test LSP rename and signature help features"""

import subprocess
import json
import sys
import os

def start_lsp():
    """Start the LSP server"""
    lsp_path = os.path.join(os.path.dirname(__file__), '../../target/debug/zen')
    return subprocess.Popen(
        [lsp_path, '--lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

def send_request(proc, method, params):
    """Send a JSON-RPC request to the LSP"""
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    }
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

def read_response(proc):
    """Read a JSON-RPC response from the LSP"""
    # Read Content-Length header
    line = proc.stdout.readline().decode()
    if not line.startswith("Content-Length:"):
        return None
    length = int(line.split(":")[1].strip())

    # Skip empty line
    proc.stdout.readline()

    # Read content
    content = proc.stdout.read(length).decode()
    return json.loads(content)

def test_signature_help():
    """Test signature help feature"""
    print("Testing Signature Help...")

    proc = start_lsp()

    # Initialize
    send_request(proc, "initialize", {
        "capabilities": {}
    })
    response = read_response(proc)

    # Open document
    test_file = os.path.join(os.path.dirname(__file__), 'test_rename.zen')
    with open(test_file, 'r') as f:
        content = f.read()

    send_request(proc, "textDocument/didOpen", {
        "textDocument": {
            "uri": f"file://{test_file}",
            "languageId": "zen",
            "version": 1,
            "text": content
        }
    })

    # Request signature help at position inside function call
    # Line 5: "let result = add(5, 10);"
    #                          ^
    send_request(proc, "textDocument/signatureHelp", {
        "textDocument": {"uri": f"file://{test_file}"},
        "position": {"line": 5, "character": 21}
    })

    response = read_response(proc)
    if response and "result" in response:
        result = response["result"]
        if result and "signatures" in result and len(result["signatures"]) > 0:
            sig = result["signatures"][0]
            print(f"✅ Signature Help PASSED: {sig.get('label', 'N/A')}")
            if "activeParameter" in result:
                print(f"   Active parameter: {result['activeParameter']}")
            return True
        else:
            print("❌ Signature Help FAILED: No signatures returned")
            return False

    print("❌ Signature Help FAILED: Invalid response")
    return False

def main():
    print("Running LSP Advanced Feature Tests...\n")

    # First, build the LSP
    print("Building LSP server...")
    result = subprocess.run(['cargo', 'build'], capture_output=True)
    if result.returncode != 0:
        print("❌ Build failed!")
        print(result.stderr.decode())
        return 1

    # Run tests
    tests_passed = 0
    tests_total = 1

    if test_signature_help():
        tests_passed += 1

    print(f"\n{'='*50}")
    print(f"Tests passed: {tests_passed}/{tests_total}")

    if tests_passed == tests_total:
        print("🎉 All tests PASSED!")
        return 0
    else:
        print("❌ Some tests FAILED")
        return 1

if __name__ == "__main__":
    sys.exit(main())
