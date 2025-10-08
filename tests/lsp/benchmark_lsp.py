#!/usr/bin/env python3
"""LSP Performance Benchmark
Measures response times for all major LSP operations.
"""

import subprocess
import json
import time
import sys
from pathlib import Path

class LSPBenchmark:
    def __init__(self):
        self.lsp_path = Path(__file__).parent.parent.parent / "target" / "release" / "zen-lsp"
        self.test_file = Path("/tmp/bench_lsp.zen")
        self.msg_id = 0

    def next_id(self):
        self.msg_id += 1
        return self.msg_id

    def send_request(self, lsp, method, params):
        req_id = self.next_id()
        request = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params
        }
        msg = json.dumps(request)
        header = f"Content-Length: {len(msg)}\r\n\r\n"
        lsp.stdin.write((header + msg).encode())
        lsp.stdin.flush()
        return req_id

    def read_response(self, lsp):
        # Read header
        header = b""
        while b"\r\n\r\n" not in header:
            header += lsp.stdout.read(1)

        # Parse content length
        content_length = int(header.decode().split("Content-Length: ")[1].split("\r\n")[0])

        # Read body
        body = lsp.stdout.read(content_length)
        return json.loads(body.decode())

    def benchmark_operation(self, lsp, name, method, params):
        """Benchmark a single operation"""
        start = time.time()
        self.send_request(lsp, method, params)
        response = self.read_response(lsp)
        elapsed = (time.time() - start) * 1000  # Convert to ms

        success = "result" in response or response.get("result") is not None
        return elapsed, success

    def run_benchmarks(self):
        test_code = """{ Result, Option, HashMap, DynVec, get_default_allocator } = @std

divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Result.Ok(5.0)
}

greet = (name: StaticString) void {
    println("Hello")
}

main = () i32 {
    allocator ::= get_default_allocator()
    map ::= HashMap.new<StaticString, i32>(allocator)
    result = divide(10.0, 2.0)
    result ?
        | Ok(val) { println("Success") }
        | Err(msg) { println("Error") }
    return 0
}
"""

        self.test_file.write_text(test_code)

        # Start LSP server
        lsp = subprocess.Popen(
            [str(self.lsp_path), "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )

        try:
            # Initialize
            self.send_request(lsp, "initialize", {
                "processId": None,
                "rootUri": "file:///tmp",
                "capabilities": {}
            })
            self.read_response(lsp)

            # Send initialized notification
            notif = {"jsonrpc": "2.0", "method": "initialized", "params": {}}
            msg = json.dumps(notif)
            header = f"Content-Length: {len(msg)}\r\n\r\n"
            lsp.stdin.write((header + msg).encode())
            lsp.stdin.flush()

            # Open document
            self.send_request(lsp, "textDocument/didOpen", {
                "textDocument": {
                    "uri": str(self.test_file.as_uri()),
                    "languageId": "zen",
                    "version": 1,
                    "text": test_code
                }
            })

            time.sleep(0.2)  # Wait for initial analysis

            # Run benchmarks
            benchmarks = [
                ("Hover", "textDocument/hover", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "position": {"line": 2, "character": 0}
                }),
                ("Goto Definition", "textDocument/definition", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "position": {"line": 13, "character": 15}
                }),
                ("Find References", "textDocument/references", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "position": {"line": 2, "character": 0},
                    "context": {"includeDeclaration": True}
                }),
                ("Document Symbols", "textDocument/documentSymbol", {
                    "textDocument": {"uri": str(self.test_file.as_uri())}
                }),
                ("Signature Help", "textDocument/signatureHelp", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "position": {"line": 13, "character": 20}
                }),
                ("Inlay Hints", "textDocument/inlayHint", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 100, "character": 0}
                    }
                }),
                ("Completion", "textDocument/completion", {
                    "textDocument": {"uri": str(self.test_file.as_uri())},
                    "position": {"line": 10, "character": 8}
                }),
                ("Workspace Symbols", "workspace/symbol", {
                    "query": "divide"
                })
            ]

            print("=" * 70)
            print("ZEN LSP PERFORMANCE BENCHMARK")
            print("=" * 70)
            print()

            results = []
            total_time = 0

            for name, method, params in benchmarks:
                elapsed, success = self.benchmark_operation(lsp, name, method, params)
                results.append((name, elapsed, success))
                total_time += elapsed

                status = "✅" if success else "❌"
                perf_rating = "🚀" if elapsed < 50 else "⚡" if elapsed < 100 else "⏱️"
                print(f"{status} {perf_rating} {name:20} {elapsed:6.1f} ms")

            print()
            print("=" * 70)
            print(f"Average Response Time: {total_time / len(results):.1f} ms")
            print(f"Total Benchmark Time:  {total_time:.1f} ms")
            print()

            # Performance rating
            avg_time = total_time / len(results)
            if avg_time < 50:
                print("🏆 EXCELLENT - All operations under 50ms!")
            elif avg_time < 100:
                print("✅ GOOD - Operations averaging under 100ms")
            elif avg_time < 200:
                print("⚠️  OK - Some optimization opportunities exist")
            else:
                print("🐌 SLOW - Optimization needed")

            print("=" * 70)

        finally:
            lsp.terminate()
            lsp.wait()
            self.test_file.unlink(missing_ok=True)

if __name__ == "__main__":
    benchmark = LSPBenchmark()
    benchmark.run_benchmarks()
