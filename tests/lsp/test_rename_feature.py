#!/usr/bin/env python3
"""
Test Rename Symbol functionality
"""

import sys
import os

# Import the LSPClient from test_hover_types
sys.path.insert(0, os.path.dirname(__file__))
from test_hover_types import LSPClient

def test_local_variable_rename():
    """Test renaming a local variable"""
    test_file = """main = () i32 {
    value = 42
    result = value + 1
    return result
}
"""

    with open("tests/test_rename_local.zen", "w") as f:
        f.write(test_file)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        uri = f"file://{os.getcwd()}/tests/test_rename_local.zen"

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        # Rename "value" to "myValue"
        req_id = client.send_request("textDocument/rename", {
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 4},  # "value" on line 2
            "newName": "myValue"
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        assert result is not None, "Rename should return a result"
        assert "changes" in result, "Result should have 'changes'"

        changes = result["changes"]
        assert uri in changes, f"Should have edits for {uri}"

        edits = changes[uri]
        print(f"✅ Test 1 PASSED: Got {len(edits)} edits for local variable rename")

        # Verify the edits
        for edit in edits:
            print(f"  - Edit at line {edit['range']['start']['line']}: {edit['newText']}")

        return True

    finally:
        client.stop()
        if os.path.exists("tests/test_rename_local.zen"):
            os.remove("tests/test_rename_local.zen")

def test_function_rename():
    """Test renaming a function"""
    test_file = """calculate = (x: i32) i32 {
    return x + 1
}

main = () i32 {
    result = calculate(42)
    return result
}
"""

    with open("tests/test_rename_function.zen", "w") as f:
        f.write(test_file)

    client = LSPClient()
    try:
        client.start()
        client.initialize()

        uri = f"file://{os.getcwd()}/tests/test_rename_function.zen"

        # Open document
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "zen",
                "version": 1,
                "text": test_file
            }
        })

        # Rename "calculate" to "compute"
        req_id = client.send_request("textDocument/rename", {
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 0},  # "calculate" on line 1
            "newName": "compute"
        })

        response = client.wait_for_response(req_id)
        result = response.get("result")

        assert result is not None, "Rename should return a result"
        assert "changes" in result, "Result should have 'changes'"

        changes = result["changes"]
        edits = changes.get(uri, [])

        # Should rename both definition and usage
        assert len(edits) >= 2, f"Should rename definition and usage, got {len(edits)} edits"

        print(f"✅ Test 2 PASSED: Function rename with {len(edits)} edits")

        for edit in edits:
            print(f"  - Edit at line {edit['range']['start']['line']}: '{edit['newText']}'")

        return True

    finally:
        client.stop()
        if os.path.exists("tests/test_rename_function.zen"):
            os.remove("tests/test_rename_function.zen")

if __name__ == "__main__":
    print("Running LSP Rename Tests...\n")

    try:
        test_local_variable_rename()
        print()
        test_function_rename()

        print("\n🎉 All rename tests PASSED!")
        sys.exit(0)

    except AssertionError as e:
        print(f"\n❌ Test FAILED: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n💥 Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
