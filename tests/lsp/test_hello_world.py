#!/usr/bin/env python3
"""
Automated LSP testing for hello_world.zen
Tests hover, completion, and other LSP features
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
        # Try release first, then debug
        lsp_paths = ["target/release/zen-lsp", "target/debug/zen-lsp"]
        lsp_path = None
        
        for path in lsp_paths:
            if os.path.exists(path):
                lsp_path = path
                break
        
        if not lsp_path:
            print(f"❌ LSP binary not found. Building...")
            subprocess.run(["cargo", "build", "--release", "--bin", "zen-lsp"], check=True)
            lsp_path = "target/release/zen-lsp"
        
        print(f"🚀 Starting LSP: {lsp_path}")
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
        buffer = ""
        
        while time.time() - start_time < timeout:
            try:
                char = self.lsp.stdout.read(1)
                if not char:
                    time.sleep(0.01)
                    continue
                
                buffer += char
                
                # Check for Content-Length header
                if buffer.endswith("\r\n\r\n"):
                    # Extract content length
                    lines = buffer.split("\r\n")
                    content_length = None
                    for line in lines:
                        if line.startswith("Content-Length:"):
                            content_length = int(line.split(":")[1].strip())
                            break
                    
                    if content_length:
                        # Read the JSON content
                        json_content = self.lsp.stdout.read(content_length)
                        try:
                            response = json.loads(json_content)
                            if response.get("id") == self.request_id:
                                return response
                        except json.JSONDecodeError as e:
                            print(f"⚠️  JSON decode error: {e}")
                            buffer = ""
                            continue
                    buffer = ""
            except Exception as e:
                print(f"⚠️  Error reading response: {e}")
                time.sleep(0.01)
        
        return None
    
    def initialize(self):
        """Initialize LSP"""
        response = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": self.root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": {"dynamicRegistration": True},
                    "completion": {"dynamicRegistration": True},
                }
            }
        })
        
        if response and "result" in response:
            print("✅ LSP initialized")
            return True
        else:
            print(f"❌ Failed to initialize: {response}")
            return False
    
    def did_open(self, uri, content):
        """Send textDocument/didOpen notification"""
        self.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": content
            }
        })
        time.sleep(0.2)  # Give LSP time to process
    
    def hover(self, uri, line, character):
        """Test hover at position"""
        response = self.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })
        return response
    
    def completion(self, uri, line, character):
        """Test completion at position"""
        response = self.send_request("textDocument/completion", {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })
        return response
    
    def shutdown(self):
        """Shutdown LSP"""
        self.send_request("shutdown", {})
        self.send_notification("exit", {})
        if self.lsp:
            self.lsp.wait()

def test_hello_world():
    """Test hello_world.zen file"""
    tester = LSPTester()
    
    try:
        tester.start()
        if not tester.initialize():
            return False
        
        # Read hello_world.zen
        hello_world_path = Path("examples/hello_world.zen")
        if not hello_world_path.exists():
            print(f"❌ File not found: {hello_world_path}")
            return False
        
        content = hello_world_path.read_text()
        uri = f"file://{hello_world_path.absolute()}"
        
        print(f"\n📄 Testing: {hello_world_path}")
        tester.did_open(uri, content)
        
        tests_passed = 0
        tests_failed = 0
        
        # Test 1: Hover on "person" variable (line 11, character 4)
        print("\n🧪 Test 1: Hover on 'person' variable")
        response = tester.hover(uri, 10, 4)  # 0-indexed: line 11 = 10
        if response and "result" in response and response["result"]:
            result = response["result"]
            if "contents" in result:
                contents = result["contents"]
                if isinstance(contents, dict) and "value" in contents:
                    hover_text = contents["value"]
                elif isinstance(contents, str):
                    hover_text = contents
                else:
                    hover_text = str(contents)
                
                if "Person" in hover_text or "struct" in hover_text.lower():
                    print("  ✅ PASS: Shows struct definition")
                    tests_passed += 1
                else:
                    print(f"  ❌ FAIL: Expected struct definition, got: {hover_text[:100]}")
                    tests_failed += 1
            else:
                print(f"  ❌ FAIL: No contents in response: {response}")
                tests_failed += 1
        else:
            print(f"  ❌ FAIL: No hover response: {response}")
            tests_failed += 1
        
        # Test 2: Hover on "${person}" in format string (line 15, character 20)
        print("\n🧪 Test 2: Hover on '${person}' in format string")
        response = tester.hover(uri, 14, 20)  # Inside "${person"
        if response and "result" in response and response["result"]:
            result = response["result"]
            if "contents" in result:
                contents = result["contents"]
                if isinstance(contents, dict) and "value" in contents:
                    hover_text = contents["value"]
                elif isinstance(contents, str):
                    hover_text = contents
                else:
                    hover_text = str(contents)
                
                if "Person" in hover_text or "struct" in hover_text.lower():
                    print("  ✅ PASS: Shows struct definition")
                    tests_passed += 1
                else:
                    print(f"  ❌ FAIL: Expected struct definition, got: {hover_text[:100]}")
                    tests_failed += 1
            else:
                print(f"  ❌ FAIL: No contents in response: {response}")
                tests_failed += 1
        else:
            print(f"  ❌ FAIL: No hover response: {response}")
            tests_failed += 1
        
        # Test 3: Hover on "person.name" field (line 15, character 26)
        print("\n🧪 Test 3: Hover on 'person.name' field")
        response = tester.hover(uri, 14, 26)  # On "name" in "${person.name}"
        if response and "result" in response and response["result"]:
            result = response["result"]
            if "contents" in result:
                contents = result["contents"]
                if isinstance(contents, dict) and "value" in contents:
                    hover_text = contents["value"]
                elif isinstance(contents, str):
                    hover_text = contents
                else:
                    hover_text = str(contents)
                
                if "StaticString" in hover_text or "name" in hover_text.lower():
                    print("  ✅ PASS: Shows field type")
                    tests_passed += 1
                else:
                    print(f"  ❌ FAIL: Expected field type, got: {hover_text[:100]}")
                    tests_failed += 1
            else:
                print(f"  ❌ FAIL: No contents in response: {response}")
                tests_failed += 1
        else:
            print(f"  ❌ FAIL: No hover response: {response}")
            tests_failed += 1
        
        # Test 4: Completion after "person." (line 15, character 13)
        print("\n🧪 Test 4: Completion after 'person.'")
        response = tester.completion(uri, 14, 13)  # After "person."
        if response and "result" in response:
            result = response["result"]
            if isinstance(result, dict) and "items" in result:
                items = result["items"]
            elif isinstance(result, list):
                items = result
            else:
                items = []
            
            # Check if we get struct fields
            field_names = [item.get("label", "") for item in items if item.get("kind") == 5]  # FIELD = 5
            if "age" in field_names and "name" in field_names:
                print(f"  ✅ PASS: Found struct fields: {field_names}")
                tests_passed += 1
            else:
                print(f"  ❌ FAIL: Expected 'age' and 'name' fields, got: {field_names}")
                print(f"     All items: {[item.get('label') for item in items[:10]]}")
                tests_failed += 1
        else:
            print(f"  ❌ FAIL: No completion response: {response}")
            tests_failed += 1
        
        # Summary
        print(f"\n{'='*60}")
        print(f"📊 Test Results: {tests_passed} passed, {tests_failed} failed")
        print(f"{'='*60}")
        
        return tests_failed == 0
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return False
    finally:
        tester.shutdown()

if __name__ == "__main__":
    success = test_hello_world()
    sys.exit(0 if success else 1)

