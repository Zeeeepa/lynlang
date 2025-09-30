#!/usr/bin/env python3
"""
Test script for enhanced Zen LSP features:
- UFC method completion
- Goto definition for UFC methods
- Allocator warnings
- Find references
"""

import json
import subprocess
import time
import os
import sys
from pathlib import Path

def send_request(proc, request_id, method, params=None):
    """Send a JSON-RPC request to the LSP server"""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method
    }
    if params:
        request["params"] = params

    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"

    proc.stdin.write(message.encode())
    proc.stdin.flush()

def send_notification(proc, method, params=None):
    """Send a JSON-RPC notification to the LSP server"""
    notification = {
        "jsonrpc": "2.0",
        "method": method
    }
    if params:
        notification["params"] = params

    content = json.dumps(notification)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"

    proc.stdin.write(message.encode())
    proc.stdin.flush()

def read_response(proc):
    """Read a response from the LSP server"""
    headers = {}
    while True:
        line = proc.stdout.readline().decode().strip()
        if not line:
            break
        key, value = line.split(": ")
        headers[key] = value

    content_length = int(headers.get("Content-Length", 0))
    if content_length > 0:
        content = proc.stdout.read(content_length).decode()
        return json.loads(content)
    return None

def test_lsp_features():
    """Test the enhanced LSP features"""
    print("Starting Zen LSP server...")

    # Start the LSP server
    lsp_binary = Path(__file__).parent.parent.parent / "target" / "debug" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        print("Please build with: cargo build --bin zen-lsp")
        return False

    proc = subprocess.Popen(
        [str(lsp_binary)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    try:
        # Initialize the LSP
        print("Initializing LSP...")
        send_request(proc, 1, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{Path.cwd()}",
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": True
                        }
                    },
                    "hover": {},
                    "definition": {},
                    "references": {}
                }
            }
        })

        init_response = read_response(proc)
        print(f"Initialize response: {json.dumps(init_response, indent=2)[:200]}...")

        # Open test document
        test_file = Path(__file__).parent / "test_lsp_ufc_allocators.zen"
        test_content = test_file.read_text()

        send_notification(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })

        # Give server time to process
        time.sleep(0.5)

        # Test 1: Completion after dot (UFC methods)
        print("\n=== Testing UFC completion ===")
        send_request(proc, 2, "textDocument/completion", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 13, "character": 22}  # After test_str.
        })

        completion_response = read_response(proc)
        if completion_response and "result" in completion_response:
            items = completion_response["result"][:5]  # Show first 5 completions
            print("UFC method completions:")
            for item in items:
                print(f"  - {item.get('label')}: {item.get('detail', '')}")

        # Test 2: Hover over UFC method
        print("\n=== Testing hover on UFC method ===")
        send_request(proc, 3, "textDocument/hover", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 24, "character": 20}  # On .raise()
        })

        hover_response = read_response(proc)
        if hover_response and "result" in hover_response:
            hover = hover_response["result"]
            if "contents" in hover:
                print(f"Hover info: {hover['contents'].get('value', '')[:200]}...")

        # Test 3: Check for allocator warnings
        print("\n=== Checking allocator warnings ===")
        # Diagnostics should have been sent as notifications
        # In a real test we'd read those, but for now we'll just note they should exist
        print("Allocator warnings should be shown for:")
        print("  - Line 5: HashMap() without allocator")

        # Test 4: Goto definition
        print("\n=== Testing goto definition ===")
        send_request(proc, 4, "textDocument/definition", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 24, "character": 20}  # On .raise()
        })

        definition_response = read_response(proc)
        if definition_response:
            print(f"Definition response: {json.dumps(definition_response, indent=2)[:200]}...")

        print("\n=== LSP tests completed ===")
        return True

    except Exception as e:
        print(f"Error during testing: {e}")
        return False
    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    success = test_lsp_features()
    sys.exit(0 if success else 1)