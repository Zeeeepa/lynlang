#!/usr/bin/env python3
"""
LSP Hover Type Inference Tests

Tests that hover information shows correct types, not "unknown"
"""

import subprocess
import json
import time
import os
import sys

def start_lsp():
    """Start the Zen LSP server"""
    lsp_path = "target/release/zen-lsp"
    if not os.path.exists(lsp_path):
        print("Building LSP...")
        subprocess.run(["cargo", "build", "--release", "--bin", "zen-lsp"], check=True)

    proc = subprocess.Popen(
        [lsp_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    return proc

def send_lsp_request(proc, method, params):
    """Send LSP request and get response"""
    request_id = 1
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }

    message = json.dumps(request)
    content_length = len(message)

    proc.stdin.write(f"Content-Length: {content_length}\r\n\r\n{message}")
    proc.stdin.flush()

    # Read response
    while True:
        line = proc.stdout.readline()
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            proc.stdout.readline()  # Skip empty line
            response_text = proc.stdout.read(length)
            return json.loads(response_text)

def initialize_lsp(proc):
    """Initialize LSP connection"""
    response = send_lsp_request(proc, "initialize", {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    })

    send_lsp_request(proc, "initialized", {})
    return response

def test_function_signature_hover():
    """Test that function signatures show correct types"""
    test_file = """{ Result } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(5.0)
}

greet = (name: StaticString) void {
    println("Hello")
}

main = () i32 {
    return 0
}
"""

    # Write test file
    with open("tests/lsp_hover_test.zen", "w") as f:
        f.write(test_file)

    proc = start_lsp()
    try:
        initialize_lsp(proc)

        # Open document
        send_lsp_request(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen",
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        time.sleep(0.5)  # Wait for analysis

        # Test 1: Hover over "divide" function
        response = send_lsp_request(proc, "textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen"},
            "position": {"line": 2, "character": 0}
        })

        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        # Check that it shows StaticString, not "unknown"
        assert "StaticString" in hover_text, f"Expected StaticString in hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 1 PASSED: divide shows Result<f64, StaticString>")

        # Test 2: Hover over "greet" function
        response = send_lsp_request(proc, "textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen"},
            "position": {"line": 6, "character": 0}
        })

        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        assert "StaticString" in hover_text, f"Expected StaticString in hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 2 PASSED: greet shows (name: StaticString) void")

        return True

    finally:
        proc.terminate()
        proc.wait()
        os.remove("tests/lsp_hover_test.zen")

def test_pattern_match_hover():
    """Test that pattern match variables show correct types"""
    test_file = """{ Result } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(5.0)
}

main = () i32 {
    result = divide(10.0, 2.0)
    result ?
        | Ok(val) { }
        | Err(msg) { }
    return 0
}
"""

    with open("tests/lsp_pattern_test.zen", "w") as f:
        f.write(test_file)

    proc = start_lsp()
    try:
        initialize_lsp(proc)

        send_lsp_request(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/tests/lsp_pattern_test.zen",
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        time.sleep(0.5)

        # Test: Hover over "msg" in Err pattern
        response = send_lsp_request(proc, "textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_pattern_test.zen"},
            "position": {"line": 10, "character": 16}  # Position of "msg"
        })

        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        assert "StaticString" in hover_text, f"Expected StaticString in pattern hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 3 PASSED: Pattern match msg shows StaticString")

        return True

    finally:
        proc.terminate()
        proc.wait()
        os.remove("tests/lsp_pattern_test.zen")

if __name__ == "__main__":
    print("Running LSP Hover Type Tests...\n")

    try:
        test_function_signature_hover()
        test_pattern_match_hover()

        print("\n🎉 All tests PASSED!")
        sys.exit(0)

    except AssertionError as e:
        print(f"\n❌ Test FAILED: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n💥 Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
