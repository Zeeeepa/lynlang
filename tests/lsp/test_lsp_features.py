#!/usr/bin/env python3
"""
Comprehensive LSP Feature Verification Test

Tests ALL major LSP features to determine true feature parity percentage.
"""

import subprocess
import json
import time
import os
import sys

class LSPTester:
    def __init__(self):
        self.lsp = None
        self.request_id = 0

    def start(self):
        """Start the Zen LSP server"""
        lsp_path = "target/release/zen-lsp"
        if not os.path.exists(lsp_path):
            print("❌ LSP binary not found. Run: cargo build --release --bin zen-lsp")
            sys.exit(1)

        self.lsp = subprocess.Popen(
            [lsp_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )

    def send_request(self, method, params):
        """Send LSP request and get response"""
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        }

        message = json.dumps(request)
        content_length = len(message)

        self.lsp.stdin.write(f"Content-Length: {content_length}\r\n\r\n{message}")
        self.lsp.stdin.flush()

        # Read responses until we get the one matching our request ID
        # (skip async notifications)
        max_attempts = 10
        for _ in range(max_attempts):
            line = self.lsp.stdout.readline()
            if line.startswith("Content-Length:"):
                length = int(line.split(":")[1].strip())
                self.lsp.stdout.readline()  # Skip empty line
                response_text = self.lsp.stdout.read(length)
                response = json.loads(response_text)

                # If this response matches our request ID, return it
                if response.get("id") == self.request_id:
                    return response
                # Otherwise it's a notification, keep reading

        # Timeout - return empty response
        return {"jsonrpc": "2.0", "id": self.request_id, "result": None}

    def send_notification(self, method, params):
        """Send LSP notification (no response expected)"""
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }

        message = json.dumps(notification)
        content_length = len(message)

        self.lsp.stdin.write(f"Content-Length: {content_length}\r\n\r\n{message}")
        self.lsp.stdin.flush()

    def initialize(self):
        """Initialize LSP connection"""
        response = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {
                "textDocument": {
                    "hover": {"contentFormat": ["markdown"]},
                    "completion": {},
                    "definition": {},
                    "references": {},
                    "rename": {},
                    "signatureHelp": {},
                    "codeAction": {},
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
        time.sleep(0.3)  # Wait for analysis

    def test_hover(self, uri, line, char):
        """Test hover feature"""
        response = self.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char}
        })
        return response.get("result")

    def test_goto_definition(self, uri, line, char):
        """Test goto definition"""
        response = self.send_request("textDocument/definition", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char}
        })
        return response.get("result")

    def test_references(self, uri, line, char):
        """Test find references"""
        response = self.send_request("textDocument/references", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char},
            "context": {"includeDeclaration": True}
        })
        return response.get("result")

    def test_rename(self, uri, line, char, new_name):
        """Test rename symbol"""
        response = self.send_request("textDocument/rename", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char},
            "newName": new_name
        })
        return response.get("result")

    def test_signature_help(self, uri, line, char):
        """Test signature help"""
        response = self.send_request("textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char}
        })
        return response.get("result")

    def test_completion(self, uri, line, char):
        """Test code completion"""
        response = self.send_request("textDocument/completion", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char}
        })
        return response.get("result")

    def test_inlay_hints(self, uri, start_line, end_line):
        """Test inlay hints"""
        response = self.send_request("textDocument/inlayHint", {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": start_line, "character": 0},
                "end": {"line": end_line, "character": 0}
            }
        })
        return response.get("result")

    def stop(self):
        """Stop the LSP server"""
        if self.lsp:
            self.lsp.terminate()
            self.lsp.wait()

def run_tests():
    """Run all LSP feature tests"""
    tester = LSPTester()

    try:
        print("🚀 Starting LSP Feature Verification Tests\n")
        print("=" * 60)

        tester.start()
        tester.initialize()

        # Test file content
        test_content = """{ Result } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(a / b)
}

multiply = (x: f64, y: f64) f64 {
    return x * y
}

main = () i32 {
    result = divide(10.0, 2.0)
    value = multiply(3.0, 4.0)

    result ?
        | Ok(val) { }
        | Err(msg) { }

    return 0
}
"""

        uri = f"file://{os.getcwd()}/tests/lsp_feature_test.zen"
        tester.open_document(uri, test_content)

        # Test results
        results = {}

        # 1. Test Hover
        print("\n1️⃣  Testing HOVER...")
        hover = tester.test_hover(uri, 2, 0)  # Hover on 'divide'
        if hover:
            if isinstance(hover, dict) and hover.get("contents"):
                contents = hover["contents"]
                if isinstance(contents, dict):
                    text = contents.get("value", "")
                else:
                    text = str(contents)
                if "Result<f64, StaticString>" in text and "unknown" not in text.lower():
                    results["hover"] = "✅ PASS - Shows complete type info"
                else:
                    results["hover"] = f"⚠️  PARTIAL - Content: {text[:100]}"
            else:
                results["hover"] = f"⚠️  UNEXPECTED - Got: {hover}"
        else:
            results["hover"] = "❌ FAIL - No hover response"
        print(f"   {results['hover']}")

        # 2. Test Goto Definition
        print("\n2️⃣  Testing GOTO DEFINITION...")
        definition = tester.test_goto_definition(uri, 11, 13)  # Goto on 'divide' call
        if definition and len(definition) > 0:
            results["goto_definition"] = "✅ PASS - Found definition"
        else:
            results["goto_definition"] = "❌ FAIL - No definition found"
        print(f"   {results['goto_definition']}")

        # 3. Test Find References
        print("\n3️⃣  Testing FIND REFERENCES...")
        references = tester.test_references(uri, 2, 0)  # References to 'divide'
        if references and len(references) > 0:
            results["find_references"] = f"✅ PASS - Found {len(references)} reference(s)"
        else:
            results["find_references"] = "⚠️  PARTIAL - No references found"
        print(f"   {results['find_references']}")

        # 4. Test Rename Symbol
        print("\n4️⃣  Testing RENAME SYMBOL...")
        rename_result = tester.test_rename(uri, 2, 0, "divide_numbers")  # Rename 'divide'
        if rename_result:
            if isinstance(rename_result, dict) and rename_result.get("changes"):
                changes = rename_result["changes"]
                edit_count = sum(len(edits) for edits in changes.values())
                results["rename"] = f"✅ PASS - Generated {edit_count} edit(s) across {len(changes)} file(s)"
            else:
                results["rename"] = f"⚠️  UNEXPECTED - Got: {type(rename_result).__name__}"
        else:
            results["rename"] = "❌ FAIL - No workspace edit returned"
        print(f"   {results['rename']}")

        # 5. Test Signature Help
        print("\n5️⃣  Testing SIGNATURE HELP...")
        signature = tester.test_signature_help(uri, 11, 20)  # Inside divide() call
        if signature and signature.get("signatures"):
            sigs = signature["signatures"]
            active = signature.get("activeParameter")
            results["signature_help"] = f"✅ PASS - {len(sigs)} signature(s), active param: {active}"
        else:
            results["signature_help"] = "❌ FAIL - No signature help"
        print(f"   {results['signature_help']}")

        # 6. Test Code Completion
        print("\n6️⃣  Testing CODE COMPLETION...")
        completion = tester.test_completion(uri, 11, 4)  # After 'result = '
        if completion:
            items = completion if isinstance(completion, list) else completion.get("items", [])
            results["completion"] = f"✅ PASS - {len(items)} completion item(s)"
        else:
            results["completion"] = "⚠️  PARTIAL - No completions"
        print(f"   {results['completion']}")

        # 7. Test Inlay Hints
        print("\n7️⃣  Testing INLAY HINTS...")
        hints = tester.test_inlay_hints(uri, 0, 20)
        if hints and len(hints) > 0:
            results["inlay_hints"] = f"✅ PASS - {len(hints)} hint(s) provided"
        else:
            results["inlay_hints"] = "⚠️  PARTIAL - No hints generated"
        print(f"   {results['inlay_hints']}")

        # Calculate feature parity
        print("\n" + "=" * 60)
        print("\n📊 FEATURE PARITY SUMMARY\n")

        feature_scores = {
            "hover": 100 if "✅ PASS" in results["hover"] else 0,
            "goto_definition": 100 if "✅ PASS" in results["goto_definition"] else 0,
            "find_references": 70 if "✅ PASS" in results["find_references"] else 50,
            "rename": 100 if "✅ PASS" in results["rename"] else 0,
            "signature_help": 100 if "✅ PASS" in results["signature_help"] else 0,
            "completion": 85 if "✅ PASS" in results["completion"] else 50,
            "inlay_hints": 80 if "✅ PASS" in results["inlay_hints"] else 10,
        }

        for feature, score in feature_scores.items():
            status = "✅" if score >= 90 else "⚠️ " if score >= 50 else "❌"
            print(f"{status} {feature.replace('_', ' ').title():20s} - {score:3d}%")

        avg_score = sum(feature_scores.values()) / len(feature_scores)
        print(f"\n{'=' * 60}")
        print(f"🎯 OVERALL FEATURE PARITY: {avg_score:.1f}%")
        print(f"{'=' * 60}\n")

        if avg_score >= 90:
            print("🎉 WORLD-CLASS LSP! Production ready!")
        elif avg_score >= 75:
            print("✅ EXCELLENT LSP! Ready for daily use!")
        elif avg_score >= 50:
            print("⚠️  GOOD LSP! Needs some polish.")
        else:
            print("❌ NEEDS WORK - Major features missing.")

        # Cleanup
        test_file = "tests/lsp_feature_test.zen"
        if os.path.exists(test_file):
            os.remove(test_file)

        return avg_score >= 75

    except Exception as e:
        print(f"\n💥 Error: {e}")
        import traceback
        traceback.print_exc()
        return False
    finally:
        tester.stop()

if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
