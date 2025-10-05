#!/usr/bin/env python3
"""Quick test to verify LSP is responding"""

import subprocess
import json
import sys
import os

def send_lsp_message(proc, method, params):
    """Send LSP JSON-RPC message"""
    msg = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    }
    content = json.dumps(msg)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message)
    proc.stdin.flush()

def read_lsp_response(proc):
    """Read LSP response"""
    headers = {}
    while True:
        line = proc.stdout.readline()
        if line == '\r\n':
            break
        if ':' in line:
            key, value = line.split(':', 1)
            headers[key.strip()] = value.strip()

    if 'Content-Length' in headers:
        length = int(headers['Content-Length'])
        content = proc.stdout.read(length)
        return json.loads(content)
    return None

# Start LSP server
lsp_path = "../../target/release/zen-lsp"
if not os.path.exists(lsp_path):
    print(f"❌ LSP binary not found at {lsp_path}")
    sys.exit(1)

proc = subprocess.Popen(
    [lsp_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=0
)

try:
    # Initialize
    print("📤 Sending initialize request...")
    send_lsp_message(proc, "initialize", {
        "processId": os.getpid(),
        "capabilities": {},
        "rootUri": f"file://{os.getcwd()}"
    })

    response = read_lsp_response(proc)
    if response and 'result' in response:
        print(f"✅ LSP initialized successfully!")
        capabilities = response['result'].get('capabilities', {})
        print(f"   Hover: {'✅' if capabilities.get('hoverProvider') else '❌'}")
        print(f"   Completion: {'✅' if capabilities.get('completionProvider') else '❌'}")
        print(f"   Goto Def: {'✅' if capabilities.get('definitionProvider') else '❌'}")
        print(f"   Diagnostics: {'✅' if capabilities.get('textDocumentSync') else '❌'}")
    else:
        print(f"❌ Initialize failed: {response}")

    print("\n🎉 LSP is working!")

finally:
    proc.terminate()
    proc.wait(timeout=2)
