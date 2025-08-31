#!/usr/bin/env bash

# Zen LSP Stdio Mode - Development fallback
# Simple LSP implementation using bash and existing tools

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Log to stderr for debugging
log() {
    echo "[zen-lsp-stdio] $1" >&2
}

log "Starting Zen LSP in stdio mode..."

# Read LSP messages
while IFS= read -r line; do
    # Check for Content-Length header
    if [[ "$line" =~ ^Content-Length:\ ([0-9]+) ]]; then
        content_length="${BASH_REMATCH[1]}"
        
        # Read the empty line
        read -r
        
        # Read the JSON content
        content=$(head -c "$content_length")
        
        # Parse the method from JSON (simple extraction)
        if [[ "$content" =~ \"method\":\"([^\"]+)\" ]]; then
            method="${BASH_REMATCH[1]}"
            
            # Extract id if present
            id=""
            if [[ "$content" =~ \"id\":([0-9]+) ]]; then
                id="${BASH_REMATCH[1]}"
            fi
            
            case "$method" in
                "initialize")
                    # Send initialize response
                    response='{"jsonrpc":"2.0","id":'$id',"result":{"capabilities":{"textDocumentSync":1,"hoverProvider":false,"completionProvider":false,"definitionProvider":false},"serverInfo":{"name":"zen-lsp-stdio","version":"0.1.0"}}}'
                    
                    response_length=${#response}
                    echo -ne "Content-Length: $response_length\r\n\r\n$response"
                    ;;
                    
                "initialized")
                    log "Client initialized"
                    ;;
                    
                "shutdown")
                    # Send shutdown response
                    response='{"jsonrpc":"2.0","id":'$id',"result":null}'
                    response_length=${#response}
                    echo -ne "Content-Length: $response_length\r\n\r\n$response"
                    ;;
                    
                "exit")
                    log "Exiting..."
                    exit 0
                    ;;
                    
                "textDocument/didOpen"|"textDocument/didChange")
                    # Extract URI and run syntax check
                    if [[ "$content" =~ \"uri\":\"file://([^\"]+)\" ]]; then
                        file_path="${BASH_REMATCH[1]}"
                        
                        # Run zen-check if available
                        if [ -x "$PROJECT_ROOT/zen-check.sh" ]; then
                            # Run syntax check and capture output
                            if ! "$PROJECT_ROOT/zen-check.sh" "$file_path" 2>&1 | grep -q "ERROR"; then
                                # No errors - send empty diagnostics
                                notification='{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file://'$file_path'","diagnostics":[]}}'
                            else
                                # Has errors - send basic diagnostic
                                notification='{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file://'$file_path'","diagnostics":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"severity":1,"message":"Syntax error detected"}]}}'
                            fi
                            
                            notification_length=${#notification}
                            echo -ne "Content-Length: $notification_length\r\n\r\n$notification"
                        fi
                    fi
                    ;;
                    
                "textDocument/didClose")
                    # Clear diagnostics
                    if [[ "$content" =~ \"uri\":\"([^\"]+)\" ]]; then
                        uri="${BASH_REMATCH[1]}"
                        notification='{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"'$uri'","diagnostics":[]}}'
                        notification_length=${#notification}
                        echo -ne "Content-Length: $notification_length\r\n\r\n$notification"
                    fi
                    ;;
                    
                *)
                    log "Unhandled method: $method"
                    
                    # Send method not found error if id is present
                    if [ -n "$id" ]; then
                        response='{"jsonrpc":"2.0","id":'$id',"error":{"code":-32601,"message":"Method not found"}}'
                        response_length=${#response}
                        echo -ne "Content-Length: $response_length\r\n\r\n$response"
                    fi
                    ;;
            esac
        fi
    fi
done

log "LSP stdio mode terminated"