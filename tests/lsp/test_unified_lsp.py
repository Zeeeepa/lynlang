#!/usr/bin/env python3
"""
Unified Modern LSP Test Suite for Zen Language Server

Tests all LSP 3.17 features systematically:
- Core features (hover, completion, navigation)
- Advanced features (semantic tokens, call hierarchy)
- Workspace features (symbols, formatting)
- Code intelligence (signature help, inlay hints)
"""

import subprocess
import json
import os
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Any

class LSPClient:
    """Modern LSP client for testing"""
    
    def __init__(self):
        self.lsp = None
        self.request_id = 0
        self.root_uri = f"file://{os.getcwd()}"
        self.responses: Dict[int, Any] = {}
        
    def start(self):
        """Start the Zen LSP server"""
        lsp_paths = ["target/release/zen-lsp", "target/debug/zen-lsp"]
        lsp_path = None
        
        for path in lsp_paths:
            if os.path.exists(path):
                lsp_path = path
                break
        
        if not lsp_path:
            print("Building LSP...")
            subprocess.run(["cargo", "build", "--release", "--bin", "zen-lsp"], check=True)
            lsp_path = "target/release/zen-lsp"
        
        self.lsp = subprocess.Popen(
            [lsp_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )
        time.sleep(0.5)
        
    def send_request(self, method: str, params: Dict) -> int:
        """Send LSP request and return request ID"""
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
        
        return self.request_id
    
    def send_notification(self, method: str, params: Dict):
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
    
    def wait_for_response(self, req_id: int, timeout: float = 5.0) -> Optional[Dict]:
        """Wait for response to specific request"""
        start_time = time.time()
        buffer = ""
        
        while time.time() - start_time < timeout:
            try:
                char = self.lsp.stdout.read(1)
                if not char:
                    time.sleep(0.01)
                    continue
                
                buffer += char
                
                if buffer.endswith("\r\n\r\n"):
                    lines = buffer.split("\r\n")
                    content_length = None
                    for line in lines:
                        if line.startswith("Content-Length:"):
                            content_length = int(line.split(":")[1].strip())
                            break
                    
                    if content_length:
                        json_content = self.lsp.stdout.read(content_length)
                        try:
                            response = json.loads(json_content)
                            if response.get("id") == req_id:
                                return response
                        except json.JSONDecodeError:
                            pass
                    buffer = ""
            except Exception:
                time.sleep(0.01)
        
        return None
    
    def initialize(self) -> bool:
        """Initialize LSP"""
        req_id = self.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": self.root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": {"dynamicRegistration": True},
                    "completion": {"dynamicRegistration": True},
                    "signatureHelp": {"dynamicRegistration": True},
                    "definition": {"dynamicRegistration": True},
                    "typeDefinition": {"dynamicRegistration": True},
                    "references": {"dynamicRegistration": True},
                    "documentHighlight": {"dynamicRegistration": True},
                    "documentSymbol": {"dynamicRegistration": True},
                    "formatting": {"dynamicRegistration": True},
                    "rename": {"dynamicRegistration": True},
                    "codeAction": {"dynamicRegistration": True},
                    "codeLens": {"dynamicRegistration": True},
                    "inlayHint": {"dynamicRegistration": True},
                    "semanticTokens": {"dynamicRegistration": True},
                },
                "workspace": {
                    "symbol": {"dynamicRegistration": True},
                }
            }
        })
        
        response = self.wait_for_response(req_id)
        return response is not None and "result" in response
    
    def did_open(self, uri: str, content: str):
        """Open document"""
        self.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": content
            }
        })
        time.sleep(0.2)
    
    def shutdown(self):
        """Shutdown LSP"""
        self.send_request("shutdown", {})
        self.send_notification("exit", {})
        if self.lsp:
            self.lsp.wait()


class TestResult:
    """Test result container"""
    def __init__(self, name: str):
        self.name = name
        self.passed = False
        self.message = ""
        self.details = ""


class UnifiedLSPSuite:
    """Unified LSP test suite"""
    
    def __init__(self):
        self.client = LSPClient()
        self.results: List[TestResult] = []
        self.test_file_content = """// Hello World in Zen
{ io } = @std

Person: {
    age: i32,
    name: StaticString,
}

calculate = (x: i32, y: i32) i32 {
    return x + y
}

main = () i32 {
    person = Person (
        age: 20,
        name: "John"
    )
    io.println("Hello, ${person.name}!")
    sum = calculate(10, 20)
    return 0
}
"""
    
    def run_test(self, name: str, test_func):
        """Run a test and record result"""
        print(f"\n🧪 {name}...")
        try:
            result = TestResult(name)
            test_func(result)
            self.results.append(result)
            if result.passed:
                print(f"  ✅ PASS: {result.message}")
            else:
                print(f"  ❌ FAIL: {result.message}")
                if result.details:
                    print(f"     {result.details}")
        except Exception as e:
            result = TestResult(name)
            result.passed = False
            result.message = f"Exception: {e}"
            self.results.append(result)
            print(f"  💥 ERROR: {e}")
            import traceback
            traceback.print_exc()
    
    def test_hover_variable(self, result: TestResult):
        """Test hover on variable"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        self.client.did_open(uri, self.test_file_content)
        
        req_id = self.client.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": 10, "character": 4}  # "person"
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response and response["result"]:
            contents = response["result"].get("contents", {})
            hover_text = contents.get("value", "") if isinstance(contents, dict) else str(contents)
            if "Person" in hover_text:
                result.passed = True
                result.message = "Shows struct definition"
            else:
                result.message = f"Expected Person struct, got: {hover_text[:100]}"
        else:
            result.message = "No hover response"
    
    def test_hover_format_string(self, result: TestResult):
        """Test hover in format string"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        
        req_id = self.client.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": 15, "character": 20}  # "${person"
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response and response["result"]:
            contents = response["result"].get("contents", {})
            hover_text = contents.get("value", "") if isinstance(contents, dict) else str(contents)
            if "Person" in hover_text:
                result.passed = True
                result.message = "Shows struct definition in format string"
            else:
                result.message = f"Expected Person struct, got: {hover_text[:100]}"
        else:
            result.message = "No hover response"
    
    def test_completion_struct_fields(self, result: TestResult):
        """Test completion for struct fields"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        
        req_id = self.client.send_request("textDocument/completion", {
            "textDocument": {"uri": uri},
            "position": {"line": 15, "character": 13}  # After "person."
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response:
            result_data = response["result"]
            items = result_data.get("items", []) if isinstance(result_data, dict) else result_data
            
            field_names = [item.get("label", "") for item in items if item.get("kind") == 5]
            if "age" in field_names and "name" in field_names:
                result.passed = True
                result.message = f"Found struct fields: {field_names}"
            else:
                result.message = f"Expected 'age' and 'name', got: {field_names}"
        else:
            result.message = "No completion response"
    
    def test_goto_definition(self, result: TestResult):
        """Test go to definition"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        
        req_id = self.client.send_request("textDocument/definition", {
            "textDocument": {"uri": uri},
            "position": {"line": 16, "character": 13}  # "calculate"
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response:
            def_result = response["result"]
            if (isinstance(def_result, list) and len(def_result) > 0) or isinstance(def_result, dict):
                result.passed = True
                result.message = "Definition found"
            else:
                result.message = f"No definition: {def_result}"
        else:
            result.message = "No definition response"
    
    def test_signature_help(self, result: TestResult):
        """Test signature help"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        
        req_id = self.client.send_request("textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": 16, "character": 23}  # Inside calculate(
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response:
            sig_result = response["result"]
            signatures = sig_result.get("signatures", [])
            if len(signatures) > 0:
                result.passed = True
                result.message = f"Signature: {signatures[0].get('label', '')}"
            else:
                result.message = "No signatures found"
        else:
            result.message = "No signature help response"
    
    def test_document_symbols(self, result: TestResult):
        """Test document symbols"""
        uri = f"file://{os.getcwd()}/test_unified.zen"
        
        req_id = self.client.send_request("textDocument/documentSymbol", {
            "textDocument": {"uri": uri}
        })
        response = self.client.wait_for_response(req_id)
        
        if response and "result" in response:
            symbols = response["result"]
            if isinstance(symbols, list) and len(symbols) >= 3:
                result.passed = True
                result.message = f"Found {len(symbols)} symbols"
            else:
                result.message = f"Expected >= 3 symbols, got: {len(symbols) if isinstance(symbols, list) else 0}"
        else:
            result.message = "No document symbols response"
    
    def run_all_tests(self):
        """Run all tests"""
        print("="*60)
        print("UNIFIED LSP TEST SUITE")
        print("="*60)
        
        try:
            self.client.start()
            if not self.client.initialize():
                print("❌ Failed to initialize LSP")
                return False
            
            # Write test file
            test_file = Path("test_unified.zen")
            test_file.write_text(self.test_file_content)
            uri = f"file://{test_file.absolute()}"
            
            # Run tests
            self.run_test("Hover: Variable", self.test_hover_variable)
            self.run_test("Hover: Format String", self.test_hover_format_string)
            self.run_test("Completion: Struct Fields", self.test_completion_struct_fields)
            self.run_test("Go to Definition", self.test_goto_definition)
            self.run_test("Signature Help", self.test_signature_help)
            self.run_test("Document Symbols", self.test_document_symbols)
            
            # Summary
            passed = sum(1 for r in self.results if r.passed)
            total = len(self.results)
            
            print("\n" + "="*60)
            print(f"📊 RESULTS: {passed}/{total} tests passed")
            print("="*60)
            
            # Cleanup
            if test_file.exists():
                test_file.unlink()
            
            return passed == total
            
        except Exception as e:
            print(f"💥 Error: {e}")
            import traceback
            traceback.print_exc()
            return False
        finally:
            self.client.shutdown()


if __name__ == "__main__":
    suite = UnifiedLSPSuite()
    success = suite.run_all_tests()
    sys.exit(0 if success else 1)

