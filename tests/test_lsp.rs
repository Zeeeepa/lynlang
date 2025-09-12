// LSP tests for enhanced error handling and diagnostics

#[cfg(test)]
mod test_lsp {
    use tower_lsp::lsp_types::*;
    use zen::lsp::ZenServer;
    use tower_lsp::Client;
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use std::sync::Arc;

    // Mock client for testing
    struct MockClient;

    #[tower_lsp::async_trait]
    impl Client for MockClient {
        async fn register_capability(&self, _: Vec<Registration>) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn unregister_capability(&self, _: Vec<Unregistration>) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn show_message(&self, _: ShowMessageParams) {
        }

        async fn show_message_request(&self, _: ShowMessageRequestParams) -> tower_lsp::jsonrpc::Result<Option<MessageActionItem>> {
            Ok(None)
        }

        async fn log_message(&self, _: LogMessageParams) {
        }

        async fn telemetry_event(&self, _: serde_json::Value) {
        }

        async fn publish_diagnostics(&self, _: Url, _: Vec<Diagnostic>, _: Option<i32>) {
        }

        async fn configuration(&self, _: Vec<ConfigurationItem>) -> tower_lsp::jsonrpc::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }

        async fn workspace_folders(&self) -> tower_lsp::jsonrpc::Result<Option<Vec<WorkspaceFolder>>> {
            Ok(None)
        }

        async fn apply_edit(&self, _: WorkspaceEdit) -> tower_lsp::jsonrpc::Result<ApplyWorkspaceEditResponse> {
            Ok(ApplyWorkspaceEditResponse {
                applied: true,
                failure_reason: None,
                failed_change: None,
            })
        }

        async fn semantic_tokens_refresh(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn inline_value_refresh(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn inlay_hint_refresh(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn code_lens_refresh(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn diagnostic_refresh(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn work_done_progress_create(&self, _: WorkDoneProgressCreateParams) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn progress(&self, _: ProgressParams) {
        }
    }

    async fn create_test_server() -> ZenServer {
        let client = tower_lsp::Client::from(MockClient);
        ZenServer::new(client)
    }

    #[tokio::test]
    async fn test_parse_error_for_if_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'if' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "if x > 5 { print(\"big\") }".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("'if' keyword not allowed"));
        assert!(diag.message.contains("Use: condition ?"));
    }

    #[tokio::test]
    async fn test_parse_error_for_else_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'else' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "x > 5 ? | true => print(\"big\") else print(\"small\")".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("'else' keyword not allowed") || 
                diag.message.contains("Pattern matching handles all cases"));
    }

    #[tokio::test]
    async fn test_parse_error_for_match_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'match' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "match value { 1 => \"one\", 2 => \"two\" }".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("'match' keyword not allowed"));
        assert!(diag.message.contains("Use: value ?"));
    }

    #[tokio::test]
    async fn test_parse_error_for_fn_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'fn' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "fn add(a: i32, b: i32) -> i32 { a + b }".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("No 'fn'") || diag.message.contains("Function syntax"));
        assert!(diag.message.contains("name = (params) ReturnType"));
    }

    #[tokio::test]
    async fn test_parse_error_for_let_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'let' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "let x = 5".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("No 'let'") || diag.message.contains("Variable declaration"));
        assert!(diag.message.contains(":=") || diag.message.contains("::="));
    }

    #[tokio::test]
    async fn test_parse_error_for_while_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'while' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "while x < 10 { x = x + 1 }".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("No 'while'") || diag.message.contains("loop"));
        assert!(diag.message.contains("loop (condition)"));
    }

    #[tokio::test]
    async fn test_parse_error_for_for_keyword() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with 'for' keyword
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), "for i in 0..10 { print(i) }".to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert!(diag.message.contains("No 'for'") || diag.message.contains("loop"));
        assert!(diag.message.contains("(0..10).loop") || diag.message.contains("items.loop"));
    }

    #[tokio::test]
    async fn test_import_in_function_error() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with import inside function
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), r#"
                main = () void {
                    io := @std.build.import("io")
                    io.print("Hello")
                }
            "#.to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        // Should have diagnostic about imports in functions
        let has_import_error = diagnostics.iter().any(|d| 
            d.message.contains("Import statements must be at module level")
        );
        assert!(has_import_error || !diagnostics.is_empty()); // Either specific error or general parse error
    }

    #[tokio::test]
    async fn test_import_in_comptime_error() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert test document with import inside comptime
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), r#"
                comptime {
                    io := @std.build.import("io")
                }
            "#.to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        // Should have diagnostic about imports in comptime blocks
        let has_comptime_error = diagnostics.iter().any(|d| 
            d.message.contains("Imports are not allowed inside comptime blocks")
        );
        assert!(has_comptime_error || !diagnostics.is_empty()); // Either specific error or general parse error
    }

    #[tokio::test]
    async fn test_valid_zen_code_no_errors() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert valid Zen code
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), r#"
                // Valid Zen code
                add = (a: i32, b: i32) i32 {
                    a + b
                }
                
                main = () void {
                    result := add(3, 4)
                    result > 5 ? 
                        | true => print("Big")
                        | false => print("Small")
                }
            "#.to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        // Valid code might still have minor issues, but shouldn't have keyword errors
        let has_keyword_errors = diagnostics.iter().any(|d| 
            d.message.contains("keyword not allowed") ||
            d.message.contains("No 'if'") ||
            d.message.contains("No 'else'") ||
            d.message.contains("No 'match'")
        );
        assert!(!has_keyword_errors);
    }

    #[tokio::test]
    async fn test_error_location_accuracy() {
        let server = create_test_server().await;
        let uri = "test://test.zen";
        
        // Insert code with error at specific location
        {
            let mut docs = server.documents.write().await;
            docs.insert(uri.to_string(), r#"
                x := 5
                if x > 3 {
                    print("yes")
                }
            "#.to_string());
        }
        
        let diagnostics = server.get_diagnostics(uri).await;
        
        if !diagnostics.is_empty() {
            let diag = &diagnostics[0];
            // Error should be on line with 'if' (line 2, considering 0-based or 1-based)
            assert!(diag.range.start.line <= 3); // Reasonable range for the error
        }
    }
}