#!/bin/bash
# Test LSP to see where it crashes

export RUST_BACKTRACE=1

# Create a test file
cat > /tmp/test.zen <<'EOF'
fn divide(a: f64, b: f64) Result<f64, StaticString> {
    if b == 0.0 {
        Result::Err("Division by zero")
    } else {
        Result::Ok(a / b)
    }
}
EOF

# Send LSP commands
(
  sleep 0.5

  # Initialize
  echo -n 'Content-Length: 131

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234,"rootUri":"file:///tmp","capabilities":{}}}'

  sleep 0.5

  # DidOpen
  CONTENT=$(cat /tmp/test.zen)
  CONTENT_JSON=$(echo "$CONTENT" | jq -R -s .)

  MSG="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"file:///tmp/test.zen\",\"languageId\":\"zen\",\"version\":1,\"text\":$CONTENT_JSON}}}"
  LEN=$(echo -n "$MSG" | wc -c)
  echo -e "Content-Length: $LEN\r\n\r\n$MSG"

  sleep 1

  # Hover
  MSG='{"jsonrpc":"2.0","id":100,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///tmp/test.zen"},"position":{"line":0,"character":5}}}'
  LEN=$(echo -n "$MSG" | wc -c)
  echo -e "Content-Length: $LEN\r\n\r\n$MSG"

  sleep 2
) | ./target/debug/zen-lsp 2>&1
