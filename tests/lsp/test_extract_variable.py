#!/usr/bin/env python3
"""Test extract variable code action for Zen LSP."""

import json
import subprocess
import os

lsp_path = "../../target/release/zen-lsp"
server = subprocess.Popen(
    [lsp_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL
)

def send(msg):
    content = json.dumps(msg)
    server.stdin.write(f"Content-Length: {len(content)}\r\n\r\n{content}".encode())
    server.stdin.flush()

def recv():
    headers = {}
    while True:
        line = server.stdout.readline().decode()
        if line == '\r\n':
            break
        if ':' in line:
            k, v = line.split(':', 1)
            headers[k.strip()] = v.strip()

    length = int(headers.get('Content-Length', 0))
    if length:
        return json.loads(server.stdout.read(length))
    return None

# Initialize
send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"processId": None, "rootUri": f"file://{os.getcwd()}", "capabilities": {}}})
resp = recv()
print(f"✓ Initialize")

# Initialized
send({"jsonrpc": "2.0", "method": "initialized", "params": {}})

# Open doc with code to refactor
test_uri = f"file://{os.getcwd()}/test_extract.zen"
test_code = """main = () void {
    x = 10 + 20 * 3;
    io.println(x);
}"""

send({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
    "textDocument": {
        "uri": test_uri,
        "languageId": "zen",
        "version": 1,
        "text": test_code
    }
}})

# Read diagnostics
diag = recv()

# Request code actions for the "10 + 20 * 3" expression (line 1, chars 8-19)
send({"jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction", "params": {
    "textDocument": {"uri": test_uri},
    "range": {
        "start": {"line": 1, "character": 8},
        "end": {"line": 1, "character": 19}
    },
    "context": {"diagnostics": []}
}})

resp = recv()
print(f"✓ Code action request sent")

if 'result' in resp and resp['result']:
    print(f"✓ Found {len(resp['result'])} code actions")
    for action in resp['result']:
        print(f"  - {action['title']}")
        if 'Extract' in action['title']:
            print(f"    ✓ Extract variable action available!")
            if 'edit' in action and action['edit'].get('changes'):
                edits = list(action['edit']['changes'].values())[0]
                print(f"    Edits: {len(edits)}")
                for edit in edits:
                    print(f"      {edit['newText'][:50]}")
else:
    print(f"✗ No code actions found")

server.terminate()
print("\n✅ Extract variable test completed")
