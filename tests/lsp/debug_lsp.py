#!/usr/bin/env python3
"""Debug LSP responses to see what's actually being returned"""

import subprocess
import json
import os
import time

lsp = subprocess.Popen(
    ['target/release/zen-lsp'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

def send_request(method, params):
    request = {
        'jsonrpc': '2.0',
        'id': 1,
        'method': method,
        'params': params
    }
    msg = json.dumps(request)
    lsp.stdin.write(f'Content-Length: {len(msg)}\r\n\r\n{msg}')
    lsp.stdin.flush()

    # Read response
    while True:
        line = lsp.stdout.readline()
        if line.startswith('Content-Length:'):
            length = int(line.split(':')[1].strip())
            lsp.stdout.readline()
            response = json.loads(lsp.stdout.read(length))
            print(f"\n📥 Response to {method}:")
            print(json.dumps(response, indent=2))
            return response

# Initialize
send_request('initialize', {
    'processId': os.getpid(),
    'rootUri': f'file://{os.getcwd()}',
    'capabilities': {}
})

# Open document
test_content = """{ Result } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(a / b)
}

main = () i32 {
    result = divide(10.0, 2.0)
    return 0
}
"""

uri = f"file://{os.getcwd()}/tests/debug_test.zen"

lsp.stdin.write(json.dumps({
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
}) + "\r\n")
lsp.stdin.flush()

time.sleep(0.5)

# Test hover
print("\n" + "=" * 60)
print("Testing HOVER at line 2, char 0 (on 'divide')...")
send_request("textDocument/hover", {
    "textDocument": {"uri": uri},
    "position": {"line": 2, "character": 0}
})

# Test signature help
print("\n" + "=" * 60)
print("Testing SIGNATURE HELP at line 7, char 20 (inside divide call)...")
send_request("textDocument/signatureHelp", {
    "textDocument": {"uri": uri},
    "position": {"line": 7, "character": 20}
})

# Test rename
print("\n" + "=" * 60)
print("Testing RENAME at line 2, char 0 (rename 'divide')...")
send_request("textDocument/rename", {
    "textDocument": {"uri": uri},
    "position": {"line": 2, "character": 0},
    "newName": "divide_numbers"
})

lsp.terminate()
print("\n✅ Done")
