#!/usr/bin/env python3
"""
Comprehensive LSP Feature Test Suite
Tests: Signature Help, Inlay Hints, and Rename Symbol
"""

import subprocess
import json
import time
import sys
from pathlib import Path

# ANSI colors
GREEN = '\033[92m'
RED = '\033[91m'
BLUE = '\033[94m'
RESET = '\033[0m'

class LSPClient:
    def __init__(self):
        self.lsp_process = None
        self.message_id = 0

    def start(self):
        """Start the LSP server"""
        self.lsp_process = subprocess.Popen(
            ['cargo', 'run', '--bin', 'zen-lsp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=False
        )
        time.sleep(2)  # Give server time to start

    def send_request(self, method, params):
        """Send a JSON-RPC request to the LSP server"""
        self.message_id += 1
        message = {
            "jsonrpc": "2.0",
            "id": self.message_id,
            "method": method,
            "params": params
        }

        content = json.dumps(message)
        header = f"Content-Length: {len(content)}\r\n\r\n"
        full_message = (header + content).encode('utf-8')

        self.lsp_process.stdin.write(full_message)
        self.lsp_process.stdin.flush()

        return self.read_response()

    def send_notification(self, method, params):
        """Send a JSON-RPC notification (no response expected)"""
        message = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }

        content = json.dumps(message)
        header = f"Content-Length: {len(content)}\r\n\r\n"
        full_message = (header + content).encode('utf-8')

        self.lsp_process.stdin.write(full_message)
        self.lsp_process.stdin.flush()

    def read_response(self):
        """Read a JSON-RPC response from the LSP server"""
        # Keep reading until we get a response (not a notification)
        while True:
            # Read Content-Length header
            header_line = b""
            while True:
                char = self.lsp_process.stdout.read(1)
                if not char:
                    return None
                header_line += char
                if header_line.endswith(b"\r\n\r\n"):
                    break

            header = header_line.decode('utf-8')
            content_length = None
            for line in header.split('\r\n'):
                if line.startswith('Content-Length:'):
                    content_length = int(line.split(':')[1].strip())
                    break

            if content_length is None:
                return None

            # Read content
            content = self.lsp_process.stdout.read(content_length)
            message = json.loads(content.decode('utf-8'))

            # If it's a response (has 'id'), return it
            # If it's a notification (has 'method' but no 'id'), skip it
            if 'id' in message:
                return message
            # else: it's a notification, keep reading

    def initialize(self, workspace_path):
        """Initialize the LSP server"""
        response = self.send_request('initialize', {
            'processId': None,
            'rootUri': f'file://{workspace_path}',
            'capabilities': {
                'textDocument': {
                    'signatureHelp': {
                        'signatureInformation': {
                            'parameterInformation': {
                                'labelOffsetSupport': True
                            }
                        }
                    },
                    'inlayHint': {},
                    'rename': {}
                }
            }
        })

        self.send_notification('initialized', {})
        return response

    def open_document(self, uri, content):
        """Open a document"""
        self.send_notification('textDocument/didOpen', {
            'textDocument': {
                'uri': uri,
                'languageId': 'zen',
                'version': 1,
                'text': content
            }
        })
        time.sleep(0.5)  # Wait for document to be processed

    def signature_help(self, uri, line, character):
        """Request signature help"""
        return self.send_request('textDocument/signatureHelp', {
            'textDocument': {'uri': uri},
            'position': {'line': line, 'character': character}
        })

    def inlay_hints(self, uri, start_line, end_line):
        """Request inlay hints"""
        return self.send_request('textDocument/inlayHint', {
            'textDocument': {'uri': uri},
            'range': {
                'start': {'line': start_line, 'character': 0},
                'end': {'line': end_line, 'character': 0}
            }
        })

    def rename(self, uri, line, character, new_name):
        """Request rename"""
        return self.send_request('textDocument/rename', {
            'textDocument': {'uri': uri},
            'position': {'line': line, 'character': character},
            'newName': new_name
        })

    def shutdown(self):
        """Shutdown the LSP server"""
        self.send_request('shutdown', {})
        self.send_notification('exit', {})
        self.lsp_process.wait(timeout=5)

def test_signature_help(client, workspace_path):
    """Test signature help feature"""
    print(f"\n{BLUE}Testing Signature Help...{RESET}")

    test_file = workspace_path / "test_sig_help.zen"
    test_content = """
divide = (a: f64, b: f64) Result<f64, StaticString> {
    return Ok(a / b)
}

main = () void {
    result ::= divide(10.0,
}
"""

    test_file.write_text(test_content)
    uri = f"file://{test_file}"

    client.open_document(uri, test_content)

    # Request signature help inside the function call (after "divide(10.0, ")
    response = client.signature_help(uri, 6, 26)

    print(f"  Debug: Full response = {response}")

    if response and 'result' in response and response['result']:
        sig_help = response['result']
        print(f"  Debug: sig_help = {sig_help}")
        if 'signatures' in sig_help and len(sig_help['signatures']) > 0:
            sig = sig_help['signatures'][0]
            label = sig.get('label', '')
            active_param = sig_help.get('activeParameter', None)

            print(f"  ✓ Signature: {label}")
            print(f"  ✓ Active parameter: {active_param}")

            if 'divide' in label and 'f64' in label:
                print(f"{GREEN}✅ Test PASSED: Signature help works!{RESET}")
                return True

    print(f"{RED}❌ Test FAILED: No signature help received{RESET}")
    return False

def test_inlay_hints(client, workspace_path):
    """Test inlay hints feature"""
    print(f"\n{BLUE}Testing Inlay Hints...{RESET}")

    test_file = workspace_path / "test_inlay_hints.zen"
    test_content = """
main = () void {
    x ::= 42
    y ::= 3.14
    msg ::= "hello"
}
"""

    test_file.write_text(test_content)
    uri = f"file://{test_file}"

    client.open_document(uri, test_content)

    # Request inlay hints for the entire file
    response = client.inlay_hints(uri, 0, 10)

    print(f"  Debug: Full response = {response}")

    if response and 'result' in response and response['result']:
        hints = response['result']

        print(f"  ✓ Received {len(hints)} inlay hints (type: {type(hints)})")
        print(f"  Debug: hints = {hints}")

        for hint in hints:
            if isinstance(hint, dict):
                position = hint.get('position', {})
                label = hint.get('label', '')
                line = position.get('line', -1)
                char = position.get('character', -1)

                print(f"  ✓ Hint at {line}:{char} -> {label}")
            else:
                print(f"  ✓ Hint (unexpected format): {hint}")

        # Check if we got type hints for variables
        hints_str = str(hints)
        if 'i32' in hints_str or 'i64' in hints_str or 'f64' in hints_str or 'f32' in hints_str:
            print(f"{GREEN}✅ Test PASSED: Inlay hints work!{RESET}")
            return True

    print(f"{RED}❌ Test FAILED: No inlay hints received{RESET}")
    return False

def test_rename_symbol(client, workspace_path):
    """Test rename symbol feature"""
    print(f"\n{BLUE}Testing Rename Symbol...{RESET}")

    test_file = workspace_path / "test_rename.zen"
    test_content = """
old_name = (x: i32) i32 {
    return x + 1
}

main = () void {
    result ::= old_name(5)
}
"""

    test_file.write_text(test_content)
    uri = f"file://{test_file}"

    client.open_document(uri, test_content)

    # Request rename at the function definition (line 1, character 0)
    response = client.rename(uri, 1, 0, "new_name")

    print(f"  Debug: Full response = {response}")

    if response and 'result' in response and response['result']:
        workspace_edit = response['result']

        print(f"  Debug: workspace_edit = {workspace_edit}")

        if 'changes' in workspace_edit and workspace_edit['changes']:
            changes = workspace_edit['changes']

            print(f"  ✓ Will modify {len(changes)} files")

            for file_uri, edits in changes.items():
                print(f"  ✓ {len(edits)} edits in {Path(file_uri.replace('file://', '')).name}")

                for edit in edits:
                    new_text = edit.get('newText', '')
                    range_obj = edit.get('range', {})
                    start = range_obj.get('start', {})
                    print(f"    → {start.get('line', -1)}:{start.get('character', -1)} -> '{new_text}'")

            # Check if we got edits for both the definition and the call
            total_edits = sum(len(edits) for edits in changes.values())
            if total_edits >= 2:  # At least definition + one usage
                print(f"{GREEN}✅ Test PASSED: Rename symbol works!{RESET}")
                return True

    print(f"{RED}❌ Test FAILED: No rename edits received{RESET}")
    return False

def main():
    print(f"{BLUE}=== LSP Feature Test Suite ==={RESET}")

    workspace_path = Path(__file__).parent.parent.parent.resolve()
    print(f"Workspace: {workspace_path}")

    client = LSPClient()

    try:
        print(f"\n{BLUE}Starting LSP server...{RESET}")
        client.start()

        print(f"{BLUE}Initializing...{RESET}")
        init_response = client.initialize(str(workspace_path))

        if not init_response or 'result' not in init_response:
            print(f"{RED}Failed to initialize LSP server{RESET}")
            return 1

        print(f"{GREEN}✓ LSP server initialized{RESET}")

        # Run tests
        results = []
        results.append(('Signature Help', test_signature_help(client, workspace_path)))
        results.append(('Inlay Hints', test_inlay_hints(client, workspace_path)))
        results.append(('Rename Symbol', test_rename_symbol(client, workspace_path)))

        # Summary
        print(f"\n{BLUE}=== Test Summary ==={RESET}")
        passed = sum(1 for _, result in results if result)
        total = len(results)

        for name, result in results:
            status = f"{GREEN}✅ PASSED{RESET}" if result else f"{RED}❌ FAILED{RESET}"
            print(f"  {name}: {status}")

        print(f"\n{BLUE}Total: {passed}/{total} tests passed{RESET}")

        # Shutdown
        print(f"\n{BLUE}Shutting down LSP server...{RESET}")
        client.shutdown()

        return 0 if passed == total else 1

    except Exception as e:
        print(f"{RED}Error: {e}{RESET}")
        import traceback
        traceback.print_exc()
        return 1
    finally:
        if client.lsp_process and client.lsp_process.poll() is None:
            client.lsp_process.terminate()
            client.lsp_process.wait(timeout=5)

if __name__ == '__main__':
    sys.exit(main())
