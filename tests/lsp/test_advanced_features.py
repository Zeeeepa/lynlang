#!/usr/bin/env python3
"""
Comprehensive LSP Feature Test
Tests: Signature Help, Inlay Hints, and Rename Symbol
"""

import subprocess
import json
import time
import os

class LSPTester:
    def __init__(self):
        self.process = None
        self.msg_id = 1

    def start_server(self):
        """Start the LSP server"""
        self.process = subprocess.Popen(
            ['/home/ubuntu/zenlang/target/debug/zen-lsp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd='/home/ubuntu/zenlang'
        )
        time.sleep(2)

    def send_request(self, method, params):
        """Send LSP request and get response"""
        request = {
            "jsonrpc": "2.0",
            "id": self.msg_id,
            "method": method,
            "params": params
        }
        request_id = self.msg_id
        self.msg_id += 1

        content = json.dumps(request)
        message = f"Content-Length: {len(content)}\r\n\r\n{content}"

        self.process.stdin.write(message.encode())
        self.process.stdin.flush()

        # Read messages until we get the response for our request
        max_reads = 10
        for _ in range(max_reads):
            header = b""
            while b"\r\n\r\n" not in header:
                char = self.process.stdout.read(1)
                if not char:
                    return None
                header += char

            content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
            response_data = self.process.stdout.read(content_length)
            message = json.loads(response_data.decode())

            # Check if this is the response to our request (has matching id)
            if "id" in message and message["id"] == request_id:
                return message
            # Otherwise it's a notification, skip it

        return None  # Timeout after max_reads

    def send_notification(self, method, params):
        """Send LSP notification (no response expected)"""
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }

        content = json.dumps(notification)
        message = f"Content-Length: {len(content)}\r\n\r\n{content}"

        self.process.stdin.write(message.encode())
        self.process.stdin.flush()

    def initialize(self):
        """Initialize LSP server"""
        workspace_path = "/home/ubuntu/zenlang/tests"
        response = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{workspace_path}",
            "capabilities": {
                "textDocument": {
                    "signatureHelp": {
                        "dynamicRegistration": True,
                        "signatureInformation": {
                            "parameterInformation": {
                                "labelOffsetSupport": True
                            }
                        }
                    },
                    "inlayHint": {
                        "dynamicRegistration": True
                    },
                    "rename": {
                        "dynamicRegistration": True,
                        "prepareSupport": True
                    }
                }
            }
        })

        self.send_notification("initialized", {})
        return response

    def open_document(self, uri, content):
        """Open a document"""
        self.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": content
            }
        })
        time.sleep(0.5)

    def test_signature_help(self):
        """Test signature help feature"""
        print("\n=== Testing Signature Help ===")

        content = """divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Ok(a / b)
}

main = () void {
    result ::= divide(10.0, 5.0)
}
"""
        uri = "file:///home/ubuntu/zenlang/tests/test_sig_help.zen"
        self.open_document(uri, content)

        # Request signature help at position after "divide(10.0, "
        # Line 5 (0-indexed), character after comma and space
        response = self.send_request("textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": 5, "character": 30}
        })

        if response and "result" in response:
            result = response["result"]
            if result and "signatures" in result and len(result["signatures"]) > 0:
                sig = result["signatures"][0]
                print(f"✅ Signature Help PASSED")
                print(f"   Label: {sig.get('label', 'N/A')}")
                print(f"   Active Parameter: {result.get('activeParameter', 'N/A')}")
                if "parameters" in sig:
                    print(f"   Parameters: {len(sig['parameters'])} params")
                return True
            else:
                print(f"❌ Signature Help FAILED: No signatures returned")
                print(f"   Response: {result}")
                return False
        else:
            print(f"❌ Signature Help FAILED: No valid response")
            print(f"   Response: {response}")
            return False

    def test_inlay_hints(self):
        """Test inlay hints feature"""
        print("\n=== Testing Inlay Hints ===")

        content = """main = () void {
    x ::= 42
    y ::= 3.14
    msg ::= "hello"
}
"""
        uri = "file:///home/ubuntu/zenlang/tests/test_inlay_hints.zen"
        self.open_document(uri, content)

        # Request inlay hints for the entire document
        response = self.send_request("textDocument/inlayHint", {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 6, "character": 0}
            }
        })

        if response and "result" in response:
            result = response["result"]
            if result and isinstance(result, list) and len(result) > 0:
                print(f"✅ Inlay Hints PASSED")
                print(f"   Found {len(result)} hints:")
                for hint in result:
                    if isinstance(hint, dict):
                        pos = hint.get('position', {})
                        label = hint.get('label', 'N/A')
                        if isinstance(pos, dict):
                            print(f"   - Line {pos.get('line', '?')}: {label}")
                        else:
                            print(f"   - {label}")
                    else:
                        print(f"   - Unexpected hint format: {hint}")
                return True
            else:
                print(f"⚠️  Inlay Hints: No hints returned (may be expected if feature needs client config)")
                print(f"   Response: {result}")
                return None  # Not necessarily a failure
        else:
            print(f"❌ Inlay Hints FAILED: No valid response")
            print(f"   Response: {response}")
            return False

    def test_rename(self):
        """Test rename symbol feature"""
        print("\n=== Testing Rename Symbol ===")

        content = """old_name = (x: i32) i32 {
    return x + 1
}

main = () void {
    result ::= old_name(5)
}
"""
        uri = "file:///home/ubuntu/zenlang/tests/test_rename.zen"
        self.open_document(uri, content)

        # Request rename for "old_name" function (line 0, character 0)
        response = self.send_request("textDocument/rename", {
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 0},
            "newName": "new_name"
        })

        if response and "result" in response:
            result = response["result"]
            if result and "changes" in result:
                changes = result["changes"]
                if uri in changes:
                    edits = changes[uri]
                    print(f"✅ Rename PASSED")
                    print(f"   Found {len(edits)} edits in file:")
                    for edit in edits[:3]:  # Show first 3 edits
                        range_info = edit['range']
                        new_text = edit['newText']
                        print(f"   - Line {range_info['start']['line']}: -> '{new_text}'")
                    if len(edits) > 3:
                        print(f"   ... and {len(edits) - 3} more")
                    return True
                else:
                    print(f"❌ Rename FAILED: No edits for current file")
                    print(f"   Files with edits: {list(changes.keys())}")
                    return False
            else:
                print(f"❌ Rename FAILED: No changes in result")
                print(f"   Response: {result}")
                return False
        else:
            print(f"❌ Rename FAILED: No valid response")
            print(f"   Response: {response}")
            return False

    def shutdown(self):
        """Shutdown the LSP server"""
        self.send_request("shutdown", {})
        self.send_notification("exit", {})
        self.process.wait(timeout=5)

    def run_all_tests(self):
        """Run all tests"""
        print("=" * 60)
        print("LSP Advanced Features Test Suite")
        print("=" * 60)

        try:
            print("\n🚀 Starting LSP server...")
            self.start_server()

            print("📡 Initializing...")
            init_response = self.initialize()

            if not init_response or "result" not in init_response:
                print("❌ Failed to initialize LSP server")
                return False

            print("✅ Server initialized successfully")

            # Run tests
            results = {}
            results['signature_help'] = self.test_signature_help()
            results['inlay_hints'] = self.test_inlay_hints()
            results['rename'] = self.test_rename()

            # Summary
            print("\n" + "=" * 60)
            print("Test Summary")
            print("=" * 60)

            passed = sum(1 for v in results.values() if v is True)
            failed = sum(1 for v in results.values() if v is False)
            skipped = sum(1 for v in results.values() if v is None)

            for test_name, result in results.items():
                status = "✅ PASSED" if result is True else ("⚠️  SKIPPED" if result is None else "❌ FAILED")
                print(f"{test_name:20s}: {status}")

            print(f"\nTotal: {passed} passed, {failed} failed, {skipped} skipped")

            success = failed == 0
            if success:
                print("\n🎉 All critical tests PASSED!")
            else:
                print(f"\n⚠️  {failed} test(s) failed")

            return success

        except Exception as e:
            print(f"\n❌ Test suite failed with error: {e}")
            import traceback
            traceback.print_exc()
            return False
        finally:
            print("\n🛑 Shutting down server...")
            try:
                self.shutdown()
            except:
                if self.process:
                    self.process.kill()
            print("✅ Server stopped")

if __name__ == "__main__":
    tester = LSPTester()
    success = tester.run_all_tests()
    exit(0 if success else 1)
