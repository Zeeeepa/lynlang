#!/usr/bin/env python3
"""Test LSP rename symbol feature"""

import json
import subprocess
import time
import os
from pathlib import Path

def send_lsp_message(proc, method, params, msg_id=None):
    """Send an LSP message"""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }
    if msg_id is not None:
        request["id"] = msg_id

    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message)
    proc.stdin.flush()

def read_lsp_response(proc, timeout=2):
    """Read LSP response with timeout"""
    import select

    ready, _, _ = select.select([proc.stdout], [], [], timeout)
    if not ready:
        return None

    header = proc.stdout.readline()
    if not header.startswith("Content-Length:"):
        return None

    content_length = int(header.split(":")[1].strip())
    proc.stdout.readline()  # empty line
    content = proc.stdout.read(content_length)
    return json.loads(content)

def test_rename():
    """Test rename symbol across the document"""
    print("Testing Zen LSP Rename Symbol...")

    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    test_content = """main = () i32 {
    x = 10
    y = add(x, 20)
    z = x + y
    0
}

add = (a: i32, b: i32) i32 {
    a + b
}"""

    test_file = Path(__file__).parent / "test_rename.zen"
    test_file.write_text(test_content)
    test_uri = test_file.as_uri()

    try:
        print(f"Starting LSP server from {lsp_binary}...")
        proc = subprocess.Popen(
            [str(lsp_binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )

        # Initialize
        send_lsp_message(proc, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{Path(__file__).parent.parent.parent}",
            "capabilities": {}
        }, msg_id=1)

        init_response = read_lsp_response(proc, timeout=5)
        if not init_response:
            print("No initialize response")
            return False

        print("✓ LSP server initialized")

        # Send initialized notification
        send_lsp_message(proc, "initialized", {})
        time.sleep(0.5)

        # Open document
        send_lsp_message(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })
        time.sleep(1)

        # Request rename of variable 'x' to 'myVar'
        # Position is at line 1, character 4 (on 'x' in 'x = 10')
        send_lsp_message(proc, "textDocument/rename", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 1, "character": 4},
            "newName": "myVar"
        }, msg_id=2)

        # Read rename response
        rename_response = None
        for _ in range(5):
            response = read_lsp_response(proc, timeout=2)
            if not response:
                break

            if "id" in response and response.get("id") == 2:
                rename_response = response
                break

        if not rename_response:
            print("\n✗ No rename response received")
            proc.terminate()
            return False

        print(f"\nRename response: {json.dumps(rename_response, indent=2)}")

        # Validate the response
        if "result" in rename_response:
            result = rename_response["result"]
            if result and "changes" in result:
                changes = result["changes"]
                if test_uri in changes:
                    edits = changes[test_uri]
                    print(f"\n✓ Rename successful! Found {len(edits)} edits:")
                    for edit in edits:
                        print(f"  - Line {edit['range']['start']['line']}, "
                              f"char {edit['range']['start']['character']}: "
                              f"'{edit['newText']}'")

                    # Verify we found all 3 occurrences of 'x' (line 1, 2, 3)
                    expected_lines = {1, 2, 3}
                    actual_lines = {edit['range']['start']['line'] for edit in edits}

                    if actual_lines == expected_lines:
                        print("\n✓ All occurrences found correctly!")
                        proc.terminate()
                        return True
                    else:
                        print(f"\n✗ Expected edits on lines {expected_lines}, got {actual_lines}")
                        proc.terminate()
                        return False
                else:
                    print(f"\n✗ No changes for {test_uri}")
                    proc.terminate()
                    return False
            else:
                print("\n✗ No changes in result")
                proc.terminate()
                return False
        else:
            print("\n✗ No result in response")
            proc.terminate()
            return False

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False
    finally:
        if test_file.exists():
            test_file.unlink()

if __name__ == "__main__":
    import sys
    success = test_rename()
    sys.exit(0 if success else 1)
