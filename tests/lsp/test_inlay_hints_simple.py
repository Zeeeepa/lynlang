#!/usr/bin/env python3
import json, subprocess, os, time

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
    ["./target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL
)

try:
    send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
    line = lsp.stdout.readline().decode()
    length = int(line.split(":")[1].strip())
    lsp.stdout.readline()
    lsp.stdout.read(length)

    send_notification(lsp, "initialized", {})

    # Code with variable declarations
    test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

main = () i32 {
    x = 42
    y = add(10, 20)
    return x + y
}"""

    uri = f"file://{os.getcwd()}/test_hints.zen"
    send_notification(lsp, "textDocument/didOpen", {
        "textDocument": {"uri": uri, "languageId": "zen", "version": 1, "text": test_code}
    })

    time.sleep(0.2)

    # Request inlay hints for the whole document
    send_request(lsp, "textDocument/inlayHint", {
        "textDocument": {"uri": uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 10, "character": 0}
        }
    }, 2)

    for i in range(5):
        line = lsp.stdout.readline().decode()
        if not line.startswith("Content-Length:"):
            continue
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        content = lsp.stdout.read(length).decode()
        response = json.loads(content)
        if response.get("id") == 2:
            print("✅ Got inlay hints response!")
            print(json.dumps(response, indent=2))
            result = response.get("result")
            if result and len(result) > 0:
                print(f"\n✅ SUCCESS: {len(result)} inlay hint(s) found!")
                for hint in result:
                    print(f"  📍 Line {hint['position']['line']}: {hint['label']}")
            else:
                print("\n⚠️  No inlay hints returned")
            break

finally:
    lsp.terminate()
    lsp.wait()
