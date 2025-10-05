#!/usr/bin/env python3
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
print(f"Initialize: {resp['id']}")

# Initialized
send({"jsonrpc": "2.0", "method": "initialized", "params": {}})

# Open doc
send({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
    "textDocument": {
        "uri": f"file://{os.getcwd()}/test.zen",
        "languageId": "zen",
        "version": 1,
        "text": "main = () void {}\nhelper_function = (x: i32) i32 { x }"
    }
}})

# Read diagnostics notification
diag = recv()
print(f"Diagnostics: {diag.get('method')}")

# Workspace symbol
send({"jsonrpc": "2.0", "id": 2, "method": "workspace/symbol", "params": {"query": "function"}})
resp = recv()
print(f"Response ID: {resp.get('id')}")
if 'result' in resp:
    print(f"Symbols found: {len(resp['result'])}")
    for sym in resp['result']:
        print(f"  - {sym['name']}")
else:
    print(f"Error: {resp.get('error')}")

server.terminate()
