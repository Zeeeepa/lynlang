"""LSP JSON-RPC Client"""

import subprocess
import json
import os


class LSPClient:
    """Simple LSP JSON-RPC client for testing"""

    def __init__(self):
        lsp_path = "target/release/zen-lsp" if os.path.exists("target/release/zen-lsp") else "target/debug/zen-lsp"
        if not os.path.exists(lsp_path):
            raise RuntimeError(f"LSP not found. Run: cargo build")
        self.proc = subprocess.Popen([lsp_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.req_id = 0

    def request(self, method: str, params: dict) -> dict:
        self.req_id += 1
        self._send({"jsonrpc": "2.0", "id": self.req_id, "method": method, "params": params})
        return self._recv(self.req_id)

    def notify(self, method: str, params: dict):
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def _send(self, msg: dict):
        content = json.dumps(msg)
        self.proc.stdin.write(f"Content-Length: {len(content)}\r\n\r\n{content}".encode())
        self.proc.stdin.flush()

    def _recv(self, expected_id: int) -> dict:
        while True:
            header = b""
            while b"\r\n\r\n" not in header:
                header += self.proc.stdout.read(1)
            length = int([l for l in header.decode().split("\r\n") if "Content-Length" in l][0].split(":")[1])
            resp = json.loads(self.proc.stdout.read(length))
            if resp.get("id") == expected_id:
                return resp

    def init(self):
        resp = self.request("initialize", {"processId": os.getpid(), "rootUri": f"file://{os.getcwd()}", "capabilities": {}})
        self.notify("initialized", {})
        return "result" in resp

    def shutdown(self):
        self.request("shutdown", {})
        self.notify("exit", {})
        self.proc.wait()

    def open(self, uri: str, content: str):
        self.notify("textDocument/didOpen", {"textDocument": {"uri": uri, "languageId": "zen", "version": 1, "text": content}})

    def hover(self, uri: str, line: int, char: int):
        return self.request("textDocument/hover", {"textDocument": {"uri": uri}, "position": {"line": line, "character": char}}).get("result")

    def completion(self, uri: str, line: int, char: int):
        r = self.request("textDocument/completion", {"textDocument": {"uri": uri}, "position": {"line": line, "character": char}}).get("result")
        return r.get("items", []) if isinstance(r, dict) else r or []

    def definition(self, uri: str, line: int, char: int):
        return self.request("textDocument/definition", {"textDocument": {"uri": uri}, "position": {"line": line, "character": char}}).get("result")

    def symbols(self, uri: str):
        return self.request("textDocument/documentSymbol", {"textDocument": {"uri": uri}}).get("result") or []

    def signature(self, uri: str, line: int, char: int):
        return self.request("textDocument/signatureHelp", {"textDocument": {"uri": uri}, "position": {"line": line, "character": char}}).get("result")

    def references(self, uri: str, line: int, char: int):
        return self.request("textDocument/references", {"textDocument": {"uri": uri}, "position": {"line": line, "character": char}, "context": {"includeDeclaration": True}}).get("result") or []

    @staticmethod
    def hover_text(hover) -> str:
        """Extract text from hover response"""
        if not hover:
            return ""
        contents = hover.get("contents", {})
        if isinstance(contents, str):
            return contents
        if isinstance(contents, dict):
            return contents.get("value", "")
        if isinstance(contents, list):
            return " ".join(c.get("value", c) if isinstance(c, dict) else c for c in contents)
        return ""
