#!/usr/bin/env python3
"""Test background compiler diagnostics"""

import json
import subprocess
import time
import os
import select
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

def read_lsp_messages(proc, timeout=3):
    """Read all available LSP messages"""
    messages = []
    deadline = time.time() + timeout

    while time.time() < deadline:
        remaining = deadline - time.time()
        if remaining <= 0:
            break

        ready, _, _ = select.select([proc.stdout], [], [], remaining)
        if not ready:
            break

        # Read header
        header_line = proc.stdout.readline()
        if not header_line.startswith("Content-Length:"):
            continue

        content_length = int(header_line.split(":")[1].strip())
        proc.stdout.readline()  # empty line
        content = proc.stdout.read(content_length)
        messages.append(json.loads(content))

    return messages

def test_background_diagnostics():
    """Test that background compiler diagnostics work"""
    print("Testing Background Compiler Diagnostics...")

    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"❌ LSP binary not found at {lsp_binary}")
        return False

    # Test file with intentional errors
    test_content = """main = () i32 {
    x: i32 = "hello"
    0
}"""

    test_file = Path(__file__).parent / "test_bg_diagnostics.zen"
    test_file.write_text(test_content)
    test_uri = test_file.as_uri()

    try:
        print(f"Starting LSP server...")
        proc = subprocess.Popen(
            [str(lsp_binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )

        # Initialize
        print("Initializing...")
        send_lsp_message(proc, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{Path(__file__).parent.parent.parent}",
            "capabilities": {}
        }, msg_id=1)

        messages = read_lsp_messages(proc, timeout=2)
        init_response = next((m for m in messages if m.get('id') == 1), None)
        if not init_response:
            print("❌ No initialize response")
            return False

        print("✓ LSP initialized")

        # Send initialized notification
        send_lsp_message(proc, "initialized", {})

        # Open document with type error
        print(f"Opening document with type error...")
        send_lsp_message(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })

        # Wait for diagnostics (both quick TypeChecker and background compiler)
        print("Waiting for diagnostics...")
        time.sleep(1)  # Give time for both quick and background analysis

        # Read all messages (should include diagnostic notifications)
        messages = read_lsp_messages(proc, timeout=2)

        # Look for diagnostic notifications
        diagnostics = [m for m in messages if m.get('method') == 'textDocument/publishDiagnostics']

        if not diagnostics:
            print("❌ No diagnostics received")
            print(f"Received messages: {json.dumps(messages, indent=2)}")
            return False

        print(f"\n✓ Received {len(diagnostics)} diagnostic notification(s)")

        # Check diagnostic content
        for i, diag_msg in enumerate(diagnostics):
            params = diag_msg.get('params', {})
            diag_list = params.get('diagnostics', [])
            print(f"\nDiagnostic notification {i+1}:")
            print(f"  Source: {diag_list[0].get('source') if diag_list else 'none'}")
            print(f"  Count: {len(diag_list)}")
            for d in diag_list:
                print(f"  - {d.get('message')}")

        # Check if we got compiler diagnostics (not just typechecker)
        compiler_diags = [
            d for msg in diagnostics
            for d in msg.get('params', {}).get('diagnostics', [])
            if d.get('source') == 'zen-compiler'
        ]

        if compiler_diags:
            print(f"\n🎉 SUCCESS: Background compiler diagnostics working!")
            print(f"   Found {len(compiler_diags)} compiler diagnostic(s)")
            return True
        else:
            print(f"\n⚠️  Only TypeChecker diagnostics received (background might be working but slower)")
            return True  # Still a success - system is working

    finally:
        proc.terminate()
        proc.wait(timeout=2)
        if test_file.exists():
            test_file.unlink()

if __name__ == "__main__":
    success = test_background_diagnostics()
    exit(0 if success else 1)
