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

with open("/tmp/lsp_stderr.log", "w") as stderr_file:
    lsp = subprocess.Popen(
        ["./target/release/zen-lsp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=stderr_file
    )

    try:
        send_request(lsp, "initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}}, 1)
        line = lsp.stdout.readline().decode()
        length = int(line.split(":")[1].strip())
        lsp.stdout.readline()
        lsp.stdout.read(length)

        send_notification(lsp, "initialized", {})

        test_code = """add = (a: i32, b: i32) i32 {
    return a + b
}

main = () i32 {
    result = add(10, 20)
    return 0
}"""

        uri = f"file://{os.getcwd()}/test_sig.zen"
        send_notification(lsp, "textDocument/didOpen", {
            "textDocument": {"uri": uri, "languageId": "zen", "version": 1, "text": test_code}
        })

        time.sleep(0.2)

        send_request(lsp, "textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": 5, "character": 17}
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
                print(json.dumps(response, indent=2))
                break

    finally:
        lsp.terminate()
        lsp.wait()

print("\n=== LSP Debug Output ===")
with open("/tmp/lsp_stderr.log") as f:
    print(f.read())
