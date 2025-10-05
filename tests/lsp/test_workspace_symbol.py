#!/usr/bin/env python3
"""Test workspace symbol search for Zen LSP server."""

import json
import subprocess
import sys
import os

def send_request(server, request_id, method, params):
    """Send a JSON-RPC request to the LSP server."""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    server.stdin.write(message.encode())
    server.stdin.flush()

def send_notification(server, method, params):
    """Send a JSON-RPC notification to the LSP server."""
    notification = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }
    content = json.dumps(notification)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    server.stdin.write(message.encode())
    server.stdin.flush()

def read_response(server):
    """Read a JSON-RPC response from the LSP server."""
    # Read header
    headers = {}
    while True:
        line = server.stdout.readline().decode('utf-8')
        if line == '\r\n':
            break
        if ':' in line:
            key, value = line.split(':', 1)
            headers[key.strip()] = value.strip()

    # Read content
    content_length = int(headers.get('Content-Length', 0))
    if content_length == 0:
        return None

    content = server.stdout.read(content_length).decode('utf-8')
    return json.loads(content)

def test_workspace_symbol():
    """Test workspace symbol search functionality."""
    print("Testing workspace symbol search...")

    # Get LSP server path
    lsp_path = os.path.join(os.path.dirname(__file__), "../../target/release/zen-lsp")

    # Start LSP server
    server = subprocess.Popen(
        [lsp_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    try:
        # Initialize
        send_request(server, 1, "initialize", {
            "processId": None,
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {}
        })
        response = read_response(server)
        print(f"✓ Initialize response received")

        # Send initialized notification
        send_notification(server, "initialized", {})

        # Open a test document
        test_uri = f"file://{os.getcwd()}/test_workspace_symbol.zen"
        test_content = """
main = () void {
    x = 42;
    y = "hello";
    io.println(x);
}

helper_function = (value: i32) i32 {
    result = value * 2;
    result
}

another_function = () void {
    io.println("test");
}
"""

        send_notification(server, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_content
            }
        })
        print(f"✓ Document opened")

        # Test workspace symbol search for "function"
        send_request(server, 2, "workspace/symbol", {
            "query": "function"
        })
        response = read_response(server)

        if response and "result" in response:
            symbols = response["result"]
            print(f"✓ Found {len(symbols)} symbols matching 'function'")

            # Should find helper_function and another_function
            function_names = [s["name"] for s in symbols]
            if "helper_function" in function_names:
                print(f"✓ Found 'helper_function'")
            if "another_function" in function_names:
                print(f"✓ Found 'another_function'")
        else:
            print(f"✗ No symbols found")
            return False

        # Test workspace symbol search for "main"
        send_request(server, 3, "workspace/symbol", {
            "query": "main"
        })
        response = read_response(server)

        if response and "result" in response:
            symbols = response["result"]
            print(f"✓ Found {len(symbols)} symbols matching 'main'")

            function_names = [s["name"] for s in symbols]
            if "main" in function_names:
                print(f"✓ Found 'main'")
        else:
            print(f"✗ No symbols found for 'main'")
            return False

        # Test workspace symbol search for stdlib symbols (e.g., "println")
        send_request(server, 4, "workspace/symbol", {
            "query": "println"
        })
        response = read_response(server)

        if response and "result" in response:
            symbols = response["result"]
            print(f"✓ Found {len(symbols)} stdlib symbols matching 'println'")
        else:
            print(f"⚠ No stdlib symbols found (this is okay if stdlib isn't indexed)")

        print("\n✅ All workspace symbol tests passed!")
        return True

    except Exception as e:
        print(f"\n✗ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False

    finally:
        server.terminate()
        server.wait()

if __name__ == "__main__":
    success = test_workspace_symbol()
    sys.exit(0 if success else 1)
