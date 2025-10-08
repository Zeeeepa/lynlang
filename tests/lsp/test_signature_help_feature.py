#!/usr/bin/env python3
"""
Test Signature Help functionality
"""

import sys
import os

# Import the LSPClient
sys.path.insert(0, os.path.dirname(__file__))
from test_hover_types import LSPClient

def test_signature_help():
    """Test signature help while typing function call"""
    test_file = """compute = (x: i32, y: i32) i32 {
    return x + y
}

main = () i32 {
    result = compute(42, 10)
    return 0
}
"""

    with open("tests/test_sig_help.zen", "w") as f:
        f.write(test_file)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        uri = f"file://{os.getcwd()}/tests/test_sig_help.zen"

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        import time
        time.sleep(0.5)  # Wait for indexing

        # Check document symbols
        req_id_symbols = client.send_request("textDocument/documentSymbol", {
            "textDocument": {"uri": uri}
        })
        symbols_resp = client.wait_for_response(req_id_symbols)
        print(f"DEBUG: Document symbols = {symbols_resp.get('result')}")

        # First, test if hover works on the function name
        req_id_hover = client.send_request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 0}  # "compute" on line 0
        })
        hover_resp = client.wait_for_response(req_id_hover)
        print(f"DEBUG: Hover on 'compute' = {hover_resp.get('result')}")

        # Request signature help at position inside the function call
        # Line 5 is "    result = compute(42, 10)" which is 0-indexed line 5
        # Let's request at position of first parameter (after the opening paren)
        req_id = client.send_request("textDocument/signatureHelp", {
            "textDocument": {"uri": uri},
            "position": {"line": 5, "character": 21}  # Right after '('
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        print(f"DEBUG: Response = {response}")
        print(f"DEBUG: Result = {result}")

        if result is None:
            print("❌ No signature help returned")
            return False

        signatures = result.get("signatures", [])
        print(f"DEBUG: Signatures = {signatures}")

        assert len(signatures) > 0, f"Should have at least one signature, got: {signatures}"

        sig = signatures[0]
        print(f"✅ Got signature: {sig.get('label', 'N/A')}")

        active_param = result.get("activeParameter")
        print(f"✅ Active parameter: {active_param}")

        # Check parameters
        params = sig.get("parameters", [])
        if len(params) > 0:
            print(f"✅ Parameters: {[p.get('label') for p in params]}")

        print("✅ Test PASSED: Signature help working!")
        return True

    finally:
        client.stop()
        if os.path.exists("tests/test_sig_help.zen"):
            os.remove("tests/test_sig_help.zen")

if __name__ == "__main__":
    print("Running LSP Signature Help Tests...\n")

    try:
        test_signature_help()
        print("\n🎉 All signature help tests PASSED!")
        sys.exit(0)

    except AssertionError as e:
        print(f"\n❌ Test FAILED: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n💥 Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
