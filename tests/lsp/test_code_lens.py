#!/usr/bin/env python3
"""Test LSP code lens feature"""

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
    proc.stdout.readline()
    content = proc.stdout.read(content_length)
    return json.loads(content)

def test_code_lens():
    """Test code lens for test functions"""
    print("Testing Zen LSP Code Lens...")

    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    test_content = """main = () i32 {
    0
}

test_addition = () i32 {
    x = 2 + 2
    0
}

test_string_operations = () i32 {
    0
}

helper_function = (x: i32) i32 {
    x * 2
}"""

    test_file = Path(__file__).parent / "test_code_lens.zen"
    test_file.write_text(test_content)
    test_uri = test_file.as_uri()

    try:
        print(f"Starting LSP server from {lsp_binary}...")

        # Open stderr log file
        stderr_file = open("/tmp/zen_lsp_stderr.log", "w")

        proc = subprocess.Popen(
            [str(lsp_binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=stderr_file,
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

        # Wait for diagnostics to complete
        for _ in range(5):
            response = read_lsp_response(proc, timeout=1)
            if response and "method" in response and response["method"] == "textDocument/publishDiagnostics":
                print("✓ Received diagnostics")
                break

        time.sleep(0.5)

        # Request code lens
        send_lsp_message(proc, "textDocument/codeLens", {
            "textDocument": {"uri": test_uri}
        }, msg_id=2)

        # Read code lens response
        lens_response = None
        for _ in range(5):
            response = read_lsp_response(proc, timeout=2)
            if not response:
                break

            if "id" in response and response.get("id") == 2:
                lens_response = response
                break

        if not lens_response:
            print("\n✗ No code lens response received")
            proc.terminate()
            return False

        print(f"\nCode lens response: {json.dumps(lens_response, indent=2)}")

        # Validate the response
        if "result" in lens_response:
            result = lens_response["result"]
            if result and isinstance(result, list):
                test_lenses = [l for l in result if "test_" in l.get("command", {}).get("arguments", [None, ""])[1]]
                print(f"\n✓ Code lens working! Found {len(test_lenses)} test lenses")

                for lens in test_lenses:
                    cmd = lens.get("command", {})
                    print(f"  - {cmd.get('title')}: {cmd.get('arguments', [None, ''])[1]}")

                # Verify we found both test functions
                if len(test_lenses) == 2:
                    print("\n✓ Found all test functions!")
                    proc.terminate()
                    return True
                else:
                    print(f"\n✗ Expected 2 test lenses, got {len(test_lenses)}")
                    proc.terminate()
                    return False
            else:
                print("\n✗ Empty result")
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

        # Print stderr log
        try:
            with open("/tmp/zen_lsp_stderr.log", "r") as f:
                stderr_content = f.read()
                if stderr_content:
                    print("\n===== LSP stderr log =====")
                    print(stderr_content)
        except:
            pass

if __name__ == "__main__":
    import sys
    success = test_code_lens()
    sys.exit(0 if success else 1)
