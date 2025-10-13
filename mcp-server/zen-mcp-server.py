#!/usr/bin/env python3
"""
Zen Language MCP Server

A Model Context Protocol (MCP) server that exposes the Zen programming language
compiler, type checker, formatter, and LSP as tools for AI agents.

This allows AI agents to:
- Compile Zen code
- Type check Zen code  
- Format Zen code
- Run Zen programs
- Query the Zen LSP for code intelligence
- Access Zen standard library documentation
"""

import json
import subprocess
import tempfile
import os
from pathlib import Path
from typing import Any, Dict, List

# Path to Zen compiler binaries (configure based on installation)
ZEN_BIN = os.environ.get("ZEN_BIN", "zen")
ZEN_LSP_BIN = os.environ.get("ZEN_LSP_BIN", "zen-lsp")
ZEN_CHECK_BIN = os.environ.get("ZEN_CHECK_BIN", "zen-check")
ZEN_FORMAT_BIN = os.environ.get("ZEN_FORMAT_BIN", "zen-format")
ZEN_STDLIB = os.environ.get("ZEN_STDLIB", "./stdlib")


def zen_compile(code: str, output_name: str = "output", opt_level: str = "O0") -> Dict[str, Any]:
    """Compile Zen code to executable."""
    with tempfile.TemporaryDirectory() as tmpdir:
        source_file = Path(tmpdir) / "main.zen"
        source_file.write_text(code)
        
        output_file = Path(tmpdir) / output_name
        cmd = [ZEN_BIN, str(source_file), "-o", str(output_file)]
        
        if opt_level != "O0":
            cmd.extend([f"--opt-level={opt_level}"])
        
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        return {
            "success": result.returncode == 0,
            "executable": str(output_file) if result.returncode == 0 else None,
            "stdout": result.stdout,
            "stderr": result.stderr
        }


def zen_check(code: str) -> Dict[str, Any]:
    """Type check Zen code without compiling."""
    with tempfile.TemporaryDirectory() as tmpdir:
        source_file = Path(tmpdir) / "check.zen"
        source_file.write_text(code)
        
        result = subprocess.run(
            [ZEN_CHECK_BIN, str(source_file)],
            capture_output=True,
            text=True,
            timeout=10
        )
        
        return {
            "valid": result.returncode == 0,
            "output": result.stdout,
            "errors": result.stderr if result.returncode != 0 else None
        }


def zen_run(code: str, timeout: int = 10) -> Dict[str, Any]:
    """Compile and run Zen code."""
    with tempfile.TemporaryDirectory() as tmpdir:
        source_file = Path(tmpdir) / "run.zen"
        source_file.write_text(code)
        
        output_file = Path(tmpdir) / "program"
        
        # Compile
        compile_result = subprocess.run(
            [ZEN_BIN, str(source_file), "-o", str(output_file)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if compile_result.returncode != 0:
            return {
                "success": False,
                "phase": "compilation",
                "error": compile_result.stderr
            }
        
        # Run
        try:
            run_result = subprocess.run(
                [str(output_file)],
                capture_output=True,
                text=True,
                timeout=timeout
            )
            
            return {
                "success": True,
                "exit_code": run_result.returncode,
                "stdout": run_result.stdout,
                "stderr": run_result.stderr
            }
        except subprocess.TimeoutExpired:
            return {
                "success": False,
                "phase": "execution",
                "error": f"Timeout after {timeout} seconds"
            }


if __name__ == "__main__":
    print("Zen MCP Server - Functions available for import")
    print("To use with MCP, implement server protocol in calling code")

