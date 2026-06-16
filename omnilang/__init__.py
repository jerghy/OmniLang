"""OmniLang — compile .omni to React, Vue, Python, Rust, and more.

Usage:
    from omnilang import compile_omni, CompileResult, CompileError
    result = compile_omni(source, target="react")
"""

__version__ = "0.2.0"

from omnilang.api import compile_omni, CompileResult, CompileError, parse_view_tree, parse_code_blocks, parse_native_ports
from omnilang.rust_bridge import RustBridge

__all__ = [
    "compile_omni",
    "CompileResult",
    "CompileError",
    "RustBridge",
    "parse_view_tree",
    "parse_code_blocks",
    "parse_native_ports",
]
