#!/usr/bin/env python3
"""Test LSP document formatting feature"""

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

def test_formatting():
    """Test document formatting"""
    print("Testing Zen LSP Document Formatting...")

    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    # Unformatted test content
    test_content = '''main = () i32 {
x = 5
y = 10
result = x + y
result > 0 ?
| true {
io.println("positive")
return 1
}
| false {
io.println("negative")
return 0
}
}'''

    test_file = Path(__file__).parent / "test_formatting.zen"
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
            "capabilities": {
                "textDocument": {
                    "formatting": {}
                }
            }
        }, msg_id=1)

        init_resp = read_lsp_response(proc)
        if not init_resp:
            print("❌ Failed to initialize LSP")
            return False
        print("✓ LSP initialized")

        # Send initialized notification
        send_lsp_message(proc, "initialized", {})
        time.sleep(0.1)

        # Open document
        send_lsp_message(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })
        time.sleep(0.1)

        # Request formatting
        print("Requesting formatting...")
        send_lsp_message(proc, "textDocument/formatting", {
            "textDocument": {
                "uri": test_uri
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": True
            }
        }, msg_id=2)

        # Read responses (may include notifications first)
        format_resp = None
        for _ in range(5):  # Try reading up to 5 responses
            resp = read_lsp_response(proc)
            if not resp:
                break

            # Skip notifications
            if 'method' in resp:
                print(f"Received notification: {resp['method']}")
                continue

            # This is the actual response
            if 'id' in resp and resp['id'] == 2:
                format_resp = resp
                break

        if not format_resp:
            print("❌ No formatting response")
            return False

        print(f"\nFormatting response:")
        print(json.dumps(format_resp, indent=2))

        if format_resp.get('result'):
            edits = format_resp['result']
            print(f"\n✓ Formatting successful! Received {len(edits)} edit(s)")

            if edits:
                formatted_text = edits[0]['newText']
                print("\nFormatted code:")
                print("=" * 50)
                print(formatted_text)
                print("=" * 50)

                # Check if properly indented
                lines = formatted_text.split('\n')
                has_indentation = any(line.startswith('    ') for line in lines)

                if has_indentation:
                    print("\n✓ Code is properly indented!")
                    return True
                else:
                    print("\n⚠ Warning: No indentation found in formatted code")
                    return False
        else:
            print("❌ Formatting failed")
            return False

    finally:
        # Cleanup
        try:
            send_lsp_message(proc, "shutdown", {}, msg_id=999)
            read_lsp_response(proc)
            send_lsp_message(proc, "exit", {})
            proc.wait(timeout=2)
        except:
            proc.kill()

        if test_file.exists():
            test_file.unlink()

if __name__ == "__main__":
    success = test_formatting()
    exit(0 if success else 1)
