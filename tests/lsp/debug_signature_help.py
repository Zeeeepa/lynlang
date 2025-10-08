#!/usr/bin/env python3
"""Debug script for signature help"""

import subprocess
import json
import sys
import time
from pathlib import Path

def send_request(lsp, msg_id_counter, method, params):
    msg_id_counter[0] += 1
    msg_id = msg_id_counter[0]
    request = {
        "jsonrpc": "2.0",
        "id": msg_id,
        "method": method,
        "params": params
    }
    msg = json.dumps(request)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()
    print(f"[SENT] {method} (id={msg_id})")
    return msg_id

def send_notification(lsp, method, params):
    notif = {"jsonrpc": "2.0", "method": method, "params": params}
    msg = json.dumps(notif)
    header = f"Content-Length: {len(msg)}\r\n\r\n"
    lsp.stdin.write((header + msg).encode())
    lsp.stdin.flush()
    print(f"[SENT] {method} (notification)")

def read_response(lsp):
    header = b""
    while b"\r\n\r\n" not in header:
        chunk = lsp.stdout.read(1)
        if not chunk:
            return None
        header += chunk
    content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])
    body = lsp.stdout.read(content_length)
    return json.loads(body.decode())

def main():
    lsp_path = Path("target/release/zen-lsp")
    if not lsp_path.exists():
        print("❌ LSP binary not found")
        return 1

    test_code = """add = (x: i32, y: i32) i32 {
    return x + y
}

divide = (a: f64, b: f64) Result<f64, String> {
    b == 0.0 ? {
        return Result.Err("Division by zero")
    }
    Result.Ok(a / b)
}

main = () void {
    result = add(10, 20)
    quotient = divide(100.0, 5.0)
}
"""

    lsp = subprocess.Popen(
        [str(lsp_path), "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    msg_id_counter = [0]

    try:
        # Initialize
        init_id = send_request(lsp, msg_id_counter, "initialize", {
            "processId": None,
            "rootUri": "file:///tmp",
            "capabilities": {
                "textDocument": {
                    "signatureHelp": {"dynamicRegistration": False}
                }
            }
        })

        resp = read_response(lsp)
        print(f"[RECV] Initialize response: {json.dumps(resp, indent=2)}")

        send_notification(lsp, "initialized", {})

        # Open document
        test_uri = "file:///tmp/test_sig.zen"
        send_notification(lsp, "textDocument/didOpen", {
            "textDocument": {
                "uri": test_uri,
                "languageId": "zen",
                "version": 1,
                "text": test_code
            }
        })

        time.sleep(0.5)

        # Drain diagnostics
        while True:
            import select
            if select.select([lsp.stdout], [], [], 0.1)[0]:
                resp = read_response(lsp)
                if resp:
                    print(f"[RECV] {resp.get('method', 'response')}")
            else:
                break

        # Check document symbols to see if 'add' is found
        print("\n=== Requesting document symbols ===")
        sym_id = send_request(lsp, msg_id_counter, "textDocument/documentSymbol", {
            "textDocument": {"uri": test_uri}
        })

        resp = read_response(lsp)
        print(f"[RECV] Document symbols:\n{json.dumps(resp, indent=2)}")

        # Request signature help inside add(10, 20)
        # Line 12: "    result = add(10, 20)"
        #                            ^cursor here
        print("\n=== Testing signature help at position (12, 21) - inside 'add(10, 20)' ===")
        sig_id = send_request(lsp, msg_id_counter, "textDocument/signatureHelp", {
            "textDocument": {"uri": test_uri},
            "position": {"line": 12, "character": 21}
        })

        resp = read_response(lsp)
        print(f"[RECV] Signature help response:\n{json.dumps(resp, indent=2)}")

        # Test inlay hints
        print("\n=== Testing inlay hints ===")
        hints_id = send_request(lsp, msg_id_counter, "textDocument/inlayHint", {
            "textDocument": {"uri": test_uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 100, "character": 0}
            }
        })

        resp = read_response(lsp)
        print(f"[RECV] Inlay hints response:\n{json.dumps(resp, indent=2)}")

        lsp.terminate()
        return 0

    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        lsp.terminate()
        return 1

if __name__ == "__main__":
    sys.exit(main())
