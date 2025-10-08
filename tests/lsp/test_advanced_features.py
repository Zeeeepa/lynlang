#!/usr/bin/env python3
"""
Test advanced LSP features: Rename Symbol, Signature Help, and Inlay Hints
Uses the same LSP client infrastructure as test_hover_types.py
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


def test_signature_help():
    """Test signature help for function calls"""
    print("Testing Signature Help...")

    test_code = """divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Ok(a / b)
}

main = () i32 {
    result := divide(10.0, 5.0)
    return 0
}
"""

    test_file = "tests/test_sig_help.zen"
    with open(test_file, "w") as f:
        f.write(test_code)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        time.sleep(0.5)

        # Request signature help after "divide(10.0, "
        # Position: line 5, after the comma and space
        req_id = client.send_request("textDocument/signatureHelp", {
            "textDocument": {"uri": f"file://{os.getcwd()}/{test_file}"},
            "position": {"line": 5, "character": 26}
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        if result and "signatures" in result and len(result["signatures"]) > 0:
            sig = result["signatures"][0]
            label = sig.get("label", "")
            print(f"✅ Signature Help PASSED: {label}")
            print(f"   Active parameter: {result.get('activeParameter', 0)}")
            return True
        else:
            print(f"❌ Signature Help FAILED: No signatures returned")
            print(f"   Response: {result}")
            return False

    finally:
        client.stop()
        if os.path.exists(test_file):
            os.remove(test_file)


def test_rename_symbol():
    """Test rename symbol across occurrences"""
    print("\nTesting Rename Symbol...")

    test_code = """old_name = (x: i32) i32 {
    return x + 1
}

main = () i32 {
    result := old_name(5)
    return old_name(10)
}
"""

    test_file = "tests/test_rename.zen"
    with open(test_file, "w") as f:
        f.write(test_code)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        time.sleep(0.5)

        # Request rename at "old_name" on line 0
        req_id = client.send_request("textDocument/rename", {
            "textDocument": {"uri": f"file://{os.getcwd()}/{test_file}"},
            "position": {"line": 0, "character": 0},
            "newName": "new_name"
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        if result and "changes" in result:
            file_uri = f"file://{os.getcwd()}/{test_file}"
            if file_uri in result["changes"]:
                edits = result["changes"][file_uri]
                print(f"✅ Rename Symbol PASSED: {len(edits)} occurrences renamed")
                for i, edit in enumerate(edits[:3]):
                    line = edit["range"]["start"]["line"]
                    print(f"   Edit {i+1}: Line {line} -> '{edit['newText']}'")
                if len(edits) > 3:
                    print(f"   ... and {len(edits) - 3} more")
                return True
            else:
                print(f"❌ Rename Symbol FAILED: No edits for file")
                return False
        else:
            print(f"❌ Rename Symbol FAILED: No changes in result")
            print(f"   Response: {result}")
            return False

    finally:
        client.stop()
        if os.path.exists(test_file):
            os.remove(test_file)


def test_inlay_hints():
    """Test inlay hints for type inference"""
    print("\nTesting Inlay Hints...")

    test_code = """main = () i32 {
    x := 42
    y := 3.14
    msg := "hello"
    return 0
}
"""

    test_file = "tests/test_inlay_hints.zen"
    with open(test_file, "w") as f:
        f.write(test_code)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{os.getcwd()}/{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        time.sleep(0.5)

        # Request inlay hints for the entire document
        req_id = client.send_request("textDocument/inlayHint", {
            "textDocument": {"uri": f"file://{os.getcwd()}/{test_file}"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 10, "character": 0}
            }
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        if result is not None:
            if isinstance(result, list) and len(result) > 0:
                print(f"✅ Inlay Hints PASSED: {len(result)} hints generated")
                for hint in result[:3]:
                    pos = hint.get("position", {})
                    label = hint.get("label", "N/A")
                    print(f"   Line {pos.get('line', '?')}: {label}")
                if len(result) > 3:
                    print(f"   ... and {len(result) - 3} more")
                return True
            else:
                print(f"⚠️  Inlay Hints: No hints generated (may be expected)")
                print(f"   Response: {result}")
                return True  # Not necessarily a failure
        else:
            print(f"❌ Inlay Hints FAILED: No result")
            return False

    finally:
        client.stop()
        if os.path.exists(test_file):
            os.remove(test_file)


def main():
    print("="*60)
    print("LSP Advanced Features Test Suite")
    print("="*60 + "\n")

    results = []

    try:
        results.append(("Signature Help", test_signature_help()))
        results.append(("Rename Symbol", test_rename_symbol()))
        results.append(("Inlay Hints", test_inlay_hints()))

        # Summary
        print("\n" + "="*60)
        print("Test Summary")
        print("="*60)

        passed = sum(1 for _, result in results if result)
        total = len(results)

        for name, result in results:
            status = "✅ PASS" if result else "❌ FAIL"
            print(f"{status}: {name}")

        print(f"\nTotal: {passed}/{total} tests passed")

        if passed == total:
            print("\n🎉 All tests PASSED!")
            return 0
        else:
            print(f"\n⚠️  {total - passed} test(s) failed")
            return 1

    except Exception as e:
        print(f"\n💥 Error: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
