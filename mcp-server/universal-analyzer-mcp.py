#!/usr/bin/env python3
"""
Universal Code Analyzer MCP Server

Exposes universal code analysis capabilities through Model Context Protocol.
Supports Python, TypeScript, JavaScript, Go, Rust, Java, and more.

AI agents can use this to:
- Analyze any codebase
- Get error lists with severity ratings
- Resolve types and get hover information
- Find references and definitions
- Scan for security issues
- Calculate code metrics
"""

import json
from typing import Any, Dict, List
from universal_code_analyzer import (
    analyze_codebase,
    get_hover_info,
    find_references,
    go_to_definition,
    detect_project_languages,
    Severity
)


# ============================================================================
# MCP TOOL DEFINITIONS
# ============================================================================

TOOLS = [
    {
        "name": "analyze_codebase",
        "description": (
            "Analyze a file or directory in ANY programming language. "
            "Returns comprehensive diagnostics including errors, warnings, "
            "type issues, security vulnerabilities, and code metrics. "
            "Supports Python, TypeScript, JavaScript, Go, Rust, Java, C++, and more."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to file or directory to analyze"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language (auto-detected if omitted)",
                    "enum": ["python", "typescript", "javascript", "go", "rust", "java", "cpp", "c"]
                },
                "include_metrics": {
                    "type": "boolean",
                    "description": "Include code metrics (complexity, coverage)",
                    "default": True
                }
            },
            "required": ["path"]
        }
    },
    {
        "name": "get_error_list",
        "description": (
            "Get a filtered list of errors/warnings from code analysis. "
            "Returns diagnostics with severity ratings, locations, and fix suggestions."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to analyze"
                },
                "min_severity": {
                    "type": "string",
                    "description": "Minimum severity to include",
                    "enum": ["error", "warning", "info", "hint"],
                    "default": "warning"
                },
                "max_results": {
                    "type": "number",
                    "description": "Maximum number of diagnostics to return",
                    "default": 50
                }
            },
            "required": ["path"]
        }
    },
    {
        "name": "hover_info",
        "description": (
            "Get type information and documentation for a symbol at a specific location. "
            "Like IDE hover tooltips - shows types, signatures, and docs."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File path"
                },
                "line": {
                    "type": "number",
                    "description": "Line number (1-indexed)"
                },
                "column": {
                    "type": "number",
                    "description": "Column number (0-indexed)"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language (auto-detected if omitted)"
                }
            },
            "required": ["file", "line", "column"]
        }
    },
    {
        "name": "find_references",
        "description": (
            "Find all references to a symbol in the codebase. "
            "Returns list of locations where the symbol is used."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File containing the symbol"
                },
                "line": {
                    "type": "number",
                    "description": "Line number of symbol"
                },
                "column": {
                    "type": "number",
                    "description": "Column number of symbol"
                }
            },
            "required": ["file", "line", "column"]
        }
    },
    {
        "name": "go_to_definition",
        "description": (
            "Jump to the definition of a symbol. "
            "Returns the location where the symbol is defined."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File containing the symbol"
                },
                "line": {
                    "type": "number",
                    "description": "Line number of symbol"
                },
                "column": {
                    "type": "number",
                    "description": "Column number of symbol"
                }
            },
            "required": ["file", "line", "column"]
        }
    },
    {
        "name": "detect_languages",
        "description": (
            "Detect all programming languages used in a project directory. "
            "Returns language names and file counts."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "directory": {
                    "type": "string",
                    "description": "Project directory path"
                }
            },
            "required": ["directory"]
        }
    }
]


# ============================================================================
# TOOL IMPLEMENTATIONS
# ============================================================================

def handle_analyze_codebase(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle codebase analysis request."""
    path = args["path"]
    language = args.get("language")
    
    result = analyze_codebase(path, language)
    
    # Convert to JSON-serializable format
    return {
        "language": result.language,
        "files_analyzed": result.files_analyzed,
        "summary": result.summary,
        "diagnostics": [
            {
                "message": d.message,
                "severity": d.severity.value,
                "location": {
                    "file": d.location.file,
                    "line": d.location.line,
                    "column": d.location.column,
                    "end_line": d.location.end_line,
                    "end_column": d.location.end_column
                },
                "code": d.code,
                "source": d.source,
                "suggestion": d.suggestion
            }
            for d in result.diagnostics
        ],
        "metrics": result.metrics
    }


def handle_get_error_list(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle error list request with filtering."""
    path = args["path"]
    min_severity = args.get("min_severity", "warning")
    max_results = args.get("max_results", 50)
    
    result = analyze_codebase(path)
    
    # Filter by severity
    severity_order = {"error": 3, "warning": 2, "info": 1, "hint": 0}
    min_level = severity_order.get(min_severity, 2)
    
    filtered = [
        d for d in result.diagnostics
        if severity_order.get(d.severity.value, 0) >= min_level
    ]
    
    # Sort by severity (highest first)
    filtered.sort(key=lambda d: severity_order.get(d.severity.value, 0), reverse=True)
    
    # Limit results
    filtered = filtered[:max_results]
    
    return {
        "total_diagnostics": len(result.diagnostics),
        "filtered_count": len(filtered),
        "diagnostics": [
            {
                "message": d.message,
                "severity": d.severity.value,
                "location": f"{d.location.file}:{d.location.line}:{d.location.column}",
                "code": d.code,
                "source": d.source,
                "suggestion": d.suggestion
            }
            for d in filtered
        ]
    }


def handle_hover_info(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle hover information request."""
    file = args["file"]
    line = args["line"]
    column = args["column"]
    language = args.get("language")
    
    return get_hover_info(file, line, column, language or "unknown")


def handle_find_references(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle find references request."""
    file = args["file"]
    line = args["line"]
    column = args["column"]
    
    refs = find_references(file, line, column, "unknown")
    
    return {
        "count": len(refs),
        "references": [
            {
                "file": ref.file,
                "line": ref.line,
                "column": ref.column
            }
            for ref in refs
        ]
    }


def handle_go_to_definition(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle go to definition request."""
    file = args["file"]
    line = args["line"]
    column = args["column"]
    
    location = go_to_definition(file, line, column, "unknown")
    
    if location:
        return {
            "found": True,
            "location": {
                "file": location.file,
                "line": location.line,
                "column": location.column
            }
        }
    else:
        return {"found": False}


def handle_detect_languages(args: Dict[str, Any]) -> Dict[str, Any]:
    """Handle language detection request."""
    directory = args["directory"]
    languages = detect_project_languages(directory)
    
    return {
        "languages": languages,
        "primary_language": max(languages.items(), key=lambda x: x[1])[0] if languages else None,
        "total_files": sum(languages.values())
    }


# ============================================================================
# MCP SERVER ENTRY POINT
# ============================================================================

def handle_tool_call(tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
    """Route tool calls to appropriate handlers."""
    handlers = {
        "analyze_codebase": handle_analyze_codebase,
        "get_error_list": handle_get_error_list,
        "hover_info": handle_hover_info,
        "find_references": handle_find_references,
        "go_to_definition": handle_go_to_definition,
        "detect_languages": handle_detect_languages
    }
    
    handler = handlers.get(tool_name)
    if handler:
        try:
            return {
                "success": True,
                "data": handler(arguments)
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }
    else:
        return {
            "success": False,
            "error": f"Unknown tool: {tool_name}"
        }


if __name__ == "__main__":
    print("Universal Code Analyzer MCP Server")
    print(f"Available tools: {len(TOOLS)}")
    for tool in TOOLS:
        print(f"  - {tool['name']}: {tool['description'][:60]}...")
    
    print("\nTo use with MCP, implement server protocol in calling code")
    print("Or test directly:")
    print("  python universal-analyzer-mcp.py")

