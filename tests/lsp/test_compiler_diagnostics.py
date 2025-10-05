#!/usr/bin/env python3
"""Test LSP compiler diagnostics integration"""

import json
import subprocess
import time
import os
from pathlib import Path

def send_lsp_message(proc, method, params, msg_id=None):
    """Send an LSP message and return response"""
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
    """Try to read LSP response (simplified)"""
    time.sleep(timeout)
    return None  # For now, just check stderr for diagnostics logs

def test_compiler_diagnostics():
    """Test that compiler diagnostics are integrated into LSP"""
    print("Testing Zen LSP Compiler Diagnostics...")

    # Build LSP server first
    print("Building LSP server...")
    build_result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=Path(__file__).parent.parent.parent,
        capture_output=True,
        text=True
    )

    if build_result.returncode != 0:
        print(f"Build failed: {build_result.stderr}")
        return False

    # Check if binary exists
    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    # Create test file with intentional errors
    test_file = Path(__file__).parent / "test_diagnostics.zen"
    test_uri = test_file.as_uri()

    try:
        # Start LSP server
        print(f"Starting LSP server from {lsp_binary}...")
        proc = subprocess.Popen(
            [str(lsp_binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )

        # Initialize
        send_lsp_message(proc, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{Path(__file__).parent.parent.parent}",
            "capabilities": {}
        }, msg_id=1)

        time.sleep(1)

        # Send initialized notification
        send_lsp_message(proc, "initialized", {})

        # Open the test file
        with open(test_file, 'r') as f:
            test_content = f.read()

        send_lsp_message(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })

        # Wait for diagnostics to be computed
        print("Waiting for diagnostics...")
        time.sleep(3)

        # Check stderr for diagnostic logs
        proc.poll()
        if proc.returncode is not None:
            stderr = proc.stderr.read()
            print(f"LSP server exited with code {proc.returncode}")
            print(f"Stderr output:\n{stderr}")

            # Check if we got compilation diagnostics
            if "[LSP] Compilation error:" in stderr or "[LSP] Compilation successful" in stderr:
                print("\n✓ Compiler diagnostics are working!")
                return True
            else:
                print("\n✗ No compiler diagnostic output found")
                return False
        else:
            # Server still running, check stderr
            stderr_output = proc.stderr.read()
            print(f"LSP stderr:\n{stderr_output}")

            # Clean up
            proc.terminate()
            proc.wait(timeout=2)

            if "[LSP] Compilation" in stderr_output:
                print("\n✓ Compiler diagnostics are working!")
                return True
            else:
                print("\n✗ No compiler diagnostic output found")
                return False

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    import sys
    success = test_compiler_diagnostics()
    sys.exit(0 if success else 1)
