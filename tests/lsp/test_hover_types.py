#!/usr/bin/env python3
"""
LSP Hover Type Inference Tests - Fixed version with proper LSP protocol handling
"""

import subprocess
import json
import time
import os
import sys
import threading
import queue

class LSPClient:
    def __init__(self):
        self.proc = None
        self.request_id = 0
        self.pending_responses = {}
        self.reader_thread = None
        self.response_queue = queue.Queue()
        self.running = False

    def start(self):
        lsp_path = "target/release/zen-lsp"
        if not os.path.exists(lsp_path):
            print("Building LSP...")
            subprocess.run(["cargo", "build", "--release", "--bin", "zen-lsp"], check=True)

        self.proc = subprocess.Popen(
            [lsp_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )

        self.running = True
        self.reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self.reader_thread.start()

    def _read_loop(self):
        """Background thread to read LSP responses"""
        while self.running:
            try:
                line = self.proc.stdout.readline()
                if not line:
                    break

                if line.startswith("Content-Length:"):
                    length = int(line.split(":")[1].strip())
                    self.proc.stdout.readline()  # Skip empty line
                    response_text = self.proc.stdout.read(length)
                    response = json.loads(response_text)

                    # Check if it's a response (has id) or notification
                    if "id" in response:
                        req_id = response["id"]
                        self.response_queue.put((req_id, response))
                    # else: it's a notification (like diagnostics), ignore

            except Exception as e:
                print(f"Read error: {e}")
                break

    def send_request(self, method, params):
        """Send LSP request and return request ID"""
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        }

        message = json.dumps(request)
        content = f"Content-Length: {len(message)}\r\n\r\n{message}"
        self.proc.stdin.write(content)
        self.proc.stdin.flush()

        return self.request_id

    def send_notification(self, method, params):
        """Send LSP notification (no response expected)"""
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }

        message = json.dumps(notification)
        content = f"Content-Length: {len(message)}\r\n\r\n{message}"
        self.proc.stdin.write(content)
        self.proc.stdin.flush()

    def wait_for_response(self, req_id, timeout=10):
        """Wait for a specific response"""
        start = time.time()
        while time.time() - start < timeout:
            try:
                resp_id, response = self.response_queue.get(timeout=0.1)
                if resp_id == req_id:
                    return response
                else:
                    # Put it back if it's for a different request
                    self.response_queue.put((resp_id, response))
            except queue.Empty:
                continue

        raise TimeoutError(f"No response for request {req_id}")

    def initialize(self):
        """Initialize LSP connection"""
        req_id = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {}
        })

        response = self.wait_for_response(req_id)

        # Send initialized notification
        self.send_notification("initialized", {})

        return response

    def stop(self):
        """Stop the LSP server"""
        self.running = False
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except:
                self.proc.kill()
                self.proc.wait()

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

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen",
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        time.sleep(0.5)  # Wait for analysis

        # Test 1: Hover over "divide" function
        req_id = client.send_request("textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen"},
            "position": {"line": 2, "character": 0}
        })

        response = client.wait_for_response(req_id)
        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        # Check that it shows StaticString, not "unknown"
        assert "StaticString" in hover_text, f"Expected StaticString in hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 1 PASSED: divide shows Result<f64, StaticString>")

        # Test 2: Hover over "greet" function
        req_id = client.send_request("textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_hover_test.zen"},
            "position": {"line": 6, "character": 0}
        })

        response = client.wait_for_response(req_id)
        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        assert "StaticString" in hover_text, f"Expected StaticString in hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 2 PASSED: greet shows (name: StaticString) void")

        return True

    finally:
        client.stop()
        if os.path.exists("tests/lsp_hover_test.zen"):
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

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/tests/lsp_pattern_test.zen",
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        time.sleep(0.5)

        # Test: Hover over "msg" in Err pattern
        req_id = client.send_request("textDocument/hover", {
            "textDocument": {"uri": f"file://{os.getcwd()}/tests/lsp_pattern_test.zen"},
            "position": {"line": 10, "character": 16}  # Position of "msg"
        })

        response = client.wait_for_response(req_id)
        hover_text = response.get("result", {}).get("contents", {}).get("value", "")

        assert "StaticString" in hover_text, f"Expected StaticString in pattern hover, got: {hover_text}"
        assert "unknown" not in hover_text.lower(), f"Should not contain 'unknown': {hover_text}"

        print("✅ Test 3 PASSED: Pattern match msg shows StaticString")

        return True

    finally:
        client.stop()
        if os.path.exists("tests/lsp_pattern_test.zen"):
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
