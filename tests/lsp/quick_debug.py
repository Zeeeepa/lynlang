#!/usr/bin/env python3
"""Quick LSP debug script"""

import subprocess
import json
import sys
import os

# Start LSP
lsp = subprocess.Popen(
    ["target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=False,
    bufsize=0
)

def send_message(message):
    """Send LSP message"""
    content = json.dumps(message).encode('utf-8')
    header = f"Content-Length: {len(content)}\r\n\r\n".encode('utf-8')
    lsp.stdin.write(header + content)
    lsp.stdin.flush()

def read_message():
    """Read LSP message with timeout"""
    import select

    # Wait for data with timeout
    ready, _, _ = select.select([lsp.stdout], [], [], 5.0)
    if not ready:
        print("TIMEOUT: No response from LSP")
        return None

    # Read Content-Length header
    header = b""
    while True:
        ch = lsp.stdout.read(1)
        if not ch:
            return None
        header += ch
        if header.endswith(b"\r\n\r\n"):
            break

    # Parse length
    header_str = header.decode('utf-8')
    length_line = header_str.split('\r\n')[0]
    length = int(length_line.split(':')[1].strip())

    # Read content
    content = lsp.stdout.read(length)
    return json.loads(content.decode('utf-8'))

# Initialize
print("Sending initialize...")
send_message({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    }
})

response = read_message()
print(f"Initialize response: {response is not None}")

# Send initialized notification
send_message({
    "jsonrpc": "2.0",
    "method": "initialized",
    "params": {}
})

# Open document
test_content = """main = () i32 {
    return 0
}
"""

uri = f"file://{os.getcwd()}/test_debug.zen"
send_message({
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
        "textDocument": {
            "uri": uri,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    }
})

print("Waiting for didOpen processing...")
import time
time.sleep(0.5)

# Test hover
print("Sending hover request...")
send_message({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "textDocument/hover",
    "params": {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 0}
    }
})

# Read responses until we get the hover response (id=2)
max_attempts = 5
for attempt in range(max_attempts):
    msg = read_message()
    if msg is None:
        print(f"Attempt {attempt + 1}: No message")
        break

    # Check if this is our response (has id=2)
    if msg.get("id") == 2:
        print(f"✅ Hover response: {msg}")
        break
    else:
        print(f"Attempt {attempt + 1}: Got {msg.get('method', 'response')} (not our response)")

lsp.terminate()
lsp.wait()
