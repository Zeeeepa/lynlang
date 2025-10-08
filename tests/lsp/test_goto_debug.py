#!/usr/bin/env python3
"""Debug goto definition"""

import json
import subprocess
import os
import time

def send_request(proc, method, params, req_id):
    request = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

def send_notification(proc, method, params):
    request = {"jsonrpc": "2.0", "method": method, "params": params}
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

lsp = subprocess.Popen(
    ["./target/debug/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    line = lsp.stdout.readline().decode()
    length = int(line.split(":")[1].strip())
    lsp.stdout.readline()
    lsp.stdout.read(length)
    
    send_notification(lsp, "initialized", {})
    time.sleep(0.1)

    test_code = """greet = (name: StaticString) void {
    print(name)
}

main = () i32 {
    greet("World")
    return 0
}"""

    uri = f"file://{os.getcwd()}/test_goto.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {"uri": uri, "languageId": "zen", "version": 1, "text": test_code}
    })
    time.sleep(0.3)

    # Try goto definition on "greet" call
    print("Testing goto definition on 'greet' at line 5, col 4...")
    send_request(lsp, "textDocument/definition", {
        "textDocument": {"uri": uri},
        "position": {"line": 5, "character": 4}
    }, 2)

    for i in range(10):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)
        
        if response.get("id") == 2:
            print(f"\n✅ Response received:")
            print(json.dumps(response, indent=2))
            
            result = response.get("result")
            if result:
                if isinstance(result, list):
                    print(f"\n✅ Found {len(result)} location(s)")
                    for loc in result:
                        print(f"  - {loc.get('uri')} @ line {loc.get('range', {}).get('start', {}).get('line')}")
                else:
                    print(f"\n✅ Found location: {result.get('uri')} @ line {result.get('range', {}).get('start', {}).get('line')}")
            else:
                print("\n❌ Result is null or empty")
            break

finally:
    lsp.terminate()
    lsp.wait()
