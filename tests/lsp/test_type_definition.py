#!/usr/bin/env python3
"""
Test TypeDefinition and DocumentHighlight LSP features
"""
import subprocess
import json
import os
import time

def send_request(proc, method, params):
    """Send LSP request and read response"""
    req_id = 1
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }

    message = json.dumps(request)
    content_length = len(message)

    proc.stdin.write(f"Content-Length: {content_length}\r\n\r\n{message}")
    proc.stdin.flush()

    # Read response
    while True:
        line = proc.stdout.readline()
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            proc.stdout.readline()  # Skip empty line
            response_text = proc.stdout.read(length)
            return json.loads(response_text)

def test_type_definition():
    """Test that type definition works"""
    print("Testing TypeDefinition and DocumentHighlight...")

    # Start LSP
    proc = subprocess.Popen(
        ["target/release/zen-lsp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    try:
        # Initialize
        send_request(proc, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {}
        })
        send_request(proc, "initialized", {})

        # Open test file
        test_file = os.path.join(os.getcwd(), "tests/lsp/type_definition_test.zen")
        with open(test_file, "r") as f:
            content = f.read()

        send_request(proc, "textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": content
            }
        })

        time.sleep(0.5)

        # Test 1: TypeDefinition on 'result' variable (line 14, should jump to Result type)
        response = send_request(proc, "textDocument/typeDefinition", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 14, "character": 4}  # 'result' variable
        })

        result = response.get("result")
        if result and "uri" in result:
            print(f"✅ TypeDefinition works! Found definition at: {result['uri']}")
        else:
            print(f"⚠️  TypeDefinition returned: {result}")

        # Test 2: DocumentHighlight on 'result' (should highlight both lines 15 and 18)
        response = send_request(proc, "textDocument/documentHighlight", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 14, "character": 4}
        })

        highlights = response.get("result", [])
        if len(highlights) >= 2:
            print(f"✅ DocumentHighlight works! Found {len(highlights)} occurrences")
        else:
            print(f"⚠️  DocumentHighlight found {len(highlights)} occurrences (expected 2+)")

        print("\n✅ Tests completed!")

    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    test_type_definition()
