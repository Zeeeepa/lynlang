#!/usr/bin/env python3
"""
Comprehensive automated LSP testing and fixing
Tests all features and identifies missing functionality
"""

import subprocess
import json
import os
import sys
import time
from pathlib import Path

class LSPTester:
    def __init__(self):
        self.lsp = None
        self.request_id = 0
        self.root_uri = f"file://{os.getcwd()}"
        
    def start(self):
        """Start the Zen LSP server"""
        lsp_path = "target/release/zen-lsp"
        if not os.path.exists(lsp_path):
            print(f"❌ LSP binary not found at {lsp_path}")
            print("Building LSP...")
            subprocess.run(["cargo", "build", "--release", "--bin", "zen-lsp"], check=True)
        
        self.lsp = subprocess.Popen(
            [lsp_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )
        time.sleep(0.5)
        
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
        header = f"Content-Length: {content_length}\r\n\r\n"
        
        self.lsp.stdin.write(header + message)
        self.lsp.stdin.flush()
        
        # Read response
        return self._read_response()
    
    def send_notification(self, method, params):
        """Send LSP notification"""
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }
        
        message = json.dumps(notification)
        content_length = len(message)
        header = f"Content-Length: {content_length}\r\n\r\n"
        
        self.lsp.stdin.write(header + message)
        self.lsp.stdin.flush()
    
    def _read_response(self, timeout=5.0):
        """Read LSP response"""
        start_time = time.time()
        while time.time() - start_time < timeout:
            line = self.lsp.stdout.readline()
            if not line:
                time.sleep(0.01)
                continue
                
            if line.startswith("Content-Length:"):
                length = int(line.split(":")[1].strip())
                self.lsp.stdout.readline()  # Skip empty line
                response_text = self.lsp.stdout.read(length)
                try:
                    response = json.loads(response_text)
                    if response.get("id") == self.request_id:
                        return response
                except json.JSONDecodeError:
                    continue
        return None
    
    def initialize(self):
        """Initialize LSP"""
        response = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": self.root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": {"contentFormat": ["markdown", "plaintext"]},
                    "completion": {
                        "completionItem": {"snippetSupport": True},
                        "triggerCharacters": [".", ":", "@", "?"]
                    },
                    "signatureHelp": {},
                    "definition": {},
                    "references": {},
                    "documentSymbol": {},
                    "workspaceSymbol": {}
                },
                "workspace": {"applyEdit": True}
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
        time.sleep(0.3)  # Give server time to process
    
    def test_completion(self, uri, line, character, expected_items=None):
        """Test completion at position"""
        response = self.send_request("textDocument/completion", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })
        
        if not response or "result" not in response:
            return {"success": False, "items": [], "error": "No response"}
        
        result = response["result"]
        if isinstance(result, dict) and "items" in result:
            items = result["items"]
        elif isinstance(result, list):
            items = result
        else:
            items = []
        
        labels = [item.get("label", "") for item in items]
        
        success = True
        missing = []
        if expected_items:
            for expected in expected_items:
                if expected not in labels:
                    success = False
                    missing.append(expected)
        
        return {
            "success": success,
            "items": items,
            "labels": labels,
            "count": len(items),
            "missing": missing
        }
    
    def test_hover(self, uri, line, character):
        """Test hover at position"""
        response = self.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })
        
        if not response or "result" not in response:
            return {"success": False, "content": None}
        
        result = response["result"]
        if result and "contents" in result:
            return {"success": True, "content": result["contents"]}
        return {"success": False, "content": None}
    
    def test_definition(self, uri, line, character):
        """Test go-to-definition"""
        response = self.send_request("textDocument/definition", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })
        
        if not response or "result" not in response:
            return {"success": False, "location": None}
        
        result = response["result"]
        return {"success": result is not None, "location": result}
    
    def shutdown(self):
        """Shutdown LSP"""
        if self.lsp:
            self.send_request("shutdown", {})
            self.send_notification("exit", {})
            self.lsp.terminate()
            self.lsp.wait()

def main():
    print("=" * 70)
    print("COMPREHENSIVE LSP AUTOMATED TESTING")
    print("=" * 70)
    
    tester = LSPTester()
    
    try:
        tester.start()
        print("✅ LSP server started")
        
        init_response = tester.initialize()
        if not init_response:
            print("❌ Failed to initialize LSP")
            return 1
        print("✅ LSP initialized")
        
        # Test file
        test_file = Path("examples/hello_world.zen")
        test_content = test_file.read_text()
        uri = f"file://{os.path.abspath(test_file)}"
        
        tester.open_document(uri, test_content)
        print(f"✅ Opened {test_file}")
        
        # Test 1: Module completion for @std.
        print("\n" + "=" * 70)
        print("TEST 1: Module Completion for @std.")
        print("=" * 70)
        
        # Create a test file with @std. at cursor
        test_content_with_std = test_content.replace("{ io } = @std", "{ io } = @std.")
        test_uri = f"file://{os.path.abspath('test_completion.zen')}"
        tester.open_document(test_uri, test_content_with_std)
        
        # Find position after @std.
        line_num = 0
        for i, line in enumerate(test_content_with_std.split('\n')):
            if '@std.' in line:
                line_num = i
                char_pos = line.find('@std.') + 5
                break
        
        completion_result = tester.test_completion(
            test_uri, 
            line_num, 
            char_pos,
            expected_items=["@std.io", "@std.types", "@std.collections"]
        )
        
        print(f"Completion items found: {completion_result['count']}")
        print(f"Labels: {completion_result['labels'][:20]}...")  # First 20
        
        if completion_result['missing']:
            print(f"❌ Missing modules: {completion_result['missing']}")
        else:
            print("✅ All expected modules found")
        
        # Test 2: Hover on various symbols
        print("\n" + "=" * 70)
        print("TEST 2: Hover Information")
        print("=" * 70)
        
        hover_tests = [
            ("io", 1, 4, "Should show io module info"),
            ("Person", 4, 0, "Should show struct definition"),
            ("person", 10, 4, "Should show variable type"),
            ('"John"', 12, 10, "Should show StaticString type"),
        ]
        
        for symbol, line, char, desc in hover_tests:
            result = tester.test_hover(uri, line, char)
            status = "✅" if result['success'] else "❌"
            print(f"{status} {desc}: {symbol}")
            if result['success'] and result['content']:
                content_str = str(result['content'])[:100]
                print(f"   Content: {content_str}...")
        
        # Test 3: Go-to-definition
        print("\n" + "=" * 70)
        print("TEST 3: Go-to-Definition")
        print("=" * 70)
        
        def_tests = [
            ("io", 1, 4, "Should go to stdlib/io/io.zen"),
            ("Person", 4, 0, "Should go to Person definition"),
            ("person", 10, 4, "Should go to person definition"),
        ]
        
        for symbol, line, char, desc in def_tests:
            result = tester.test_definition(uri, line, char)
            status = "✅" if result['success'] else "❌"
            print(f"{status} {desc}: {symbol}")
            if result['success'] and result['location']:
                loc_str = str(result['location'])[:100]
                print(f"   Location: {loc_str}...")
        
        # Test 4: Completion for general symbols
        print("\n" + "=" * 70)
        print("TEST 4: General Completion")
        print("=" * 70)
        
        # Test completion in empty context
        comp_result = tester.test_completion(uri, 2, 0)
        print(f"General completion items: {comp_result['count']}")
        print(f"Sample labels: {comp_result['labels'][:10]}")
        
        print("\n" + "=" * 70)
        print("TESTING COMPLETE")
        print("=" * 70)
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return 1
    finally:
        tester.shutdown()
    
    return 0

if __name__ == "__main__":
    sys.exit(main())

