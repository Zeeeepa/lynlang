#!/usr/bin/env python3
"""Test LSP signature help feature"""

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

    # Wait for data to be available
    ready, _, _ = select.select([proc.stdout], [], [], timeout)
    if not ready:
        return None

    # Read Content-Length header
    header = proc.stdout.readline()
    if not header.startswith("Content-Length:"):
        return None

    content_length = int(header.split(":")[1].strip())

    # Read empty line
    proc.stdout.readline()

    # Read content
    content = proc.stdout.read(content_length)
    return json.loads(content)

def test_signature_help():
    """Test signature help for function calls"""
    print("Testing Zen LSP Signature Help...")

    # Build LSP server
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

    lsp_binary = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    # Create test file
    test_content = """
main = () i32 {
    io.println("test")

    x = add(10, 20)

    0
}

add = (a: i32, b: i32) i32 {
    a + b
}
"""

    test_file = Path(__file__).parent / "test_signature.zen"
    test_file.write_text(test_content)
    test_uri = test_file.as_uri()

    try:
        # Start LSP server
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
                    "signatureHelp": {
                        "dynamicRegistration": False,
                        "signatureInformation": {
                            "documentationFormat": ["plaintext"]
                        }
                    }
                }
            }
        }, msg_id=1)

        # Read initialize response
        init_response = read_lsp_response(proc, timeout=5)
        if not init_response:
            print("No initialize response")
            return False

        print(f"Initialize response: {json.dumps(init_response, indent=2)}")

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

        # Request signature help at position inside add() call
        # Position is at: x = add(10|, 20)  (after "10")
        send_lsp_message(proc, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 4, "character": 14}  # After "10"
        }, msg_id=2)

        # Read signature help response (may need to read multiple messages)
        sig_response = None
        for _ in range(5):  # Try reading up to 5 messages
            response = read_lsp_response(proc, timeout=2)
            if not response:
                break

            print(f"\nReceived message: {json.dumps(response, indent=2)}")

            # Check if this is our signature help response
            if "id" in response and response.get("id") == 2:
                sig_response = response
                break

        if not sig_response:
            print("\n✗ No signature help response received")
            proc.terminate()
            return False

        print(f"\nSignature help response: {json.dumps(sig_response, indent=2)}")

        # Check if we got signatures
        if "result" in sig_response:
            result = sig_response["result"]
            if result and "signatures" in result and len(result["signatures"]) > 0:
                print("\n✓ Signature help is working!")
                print(f"  Signature: {result['signatures'][0]['label']}")
                if "activeParameter" in result:
                    print(f"  Active parameter: {result['activeParameter']}")
                proc.terminate()
                return True
            else:
                print("\n✗ No signatures returned")
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
    success = test_signature_help()
    sys.exit(0 if success else 1)
