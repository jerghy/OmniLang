"""Rust bridge — ctypes DLL interface for omni-compiler."""
from __future__ import annotations

import ctypes
import json
from pathlib import Path

from omnilang.errors import CompilerUnavailable
from omnilang.result import CompileResult, AnalyseResult


class RustBridge:
    """Bridge to the Rust omni-compiler (DLL-only via ctypes)."""

    _dll: ctypes.CDLL | None = None
    _checked: bool = False
    _dll_search_paths: list[Path] = [
        Path(__file__).parent.parent / "omni-compiler" / "target" / "debug" / "omni_compiler.dll",
        Path(__file__).parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
    ]

    @classmethod
    def _load(cls) -> ctypes.CDLL:
        """Load the omni_compiler DLL.

        Returns:
            Loaded ctypes.CDLL instance.

        Raises:
            CompilerUnavailable: If the DLL cannot be found on any search path.
        """
        if cls._dll is not None:
            return cls._dll
        if cls._checked:
            raise CompilerUnavailable()

        for p in cls._dll_search_paths:
            if p.exists():
                cls._dll = ctypes.CDLL(str(p))
                return cls._dll

        cls._checked = True
        raise CompilerUnavailable()

    @classmethod
    def is_available(cls) -> bool:
        """Check if the Rust compiler DLL is available."""
        try:
            cls._load()
            return True
        except CompilerUnavailable:
            return False

    @classmethod
    def compile(cls, sources: dict[str, str], target: str,
                path_mapping: dict[str, str] | None = None) -> CompileResult:
        """Compile .omni source files to target code via Rust DLL.

        Calls omni_compile_multi_to_target C FFI which handles parsing,
        semantic analysis, and codegen entirely in Rust.

        Args:
            sources: {path: source_text}
            target: Target language (python, js, rust, dart, omniview, ...)
            path_mapping: {import_path: resolved_path}

        Returns:
            CompileResult with success flag, generated files, errors, and warnings.
        """
        dll = cls._load()
        func = dll.omni_compile_multi_to_target
        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        sources_bytes = json.dumps(sources).encode("utf-8")
        target_bytes = target.encode("utf-8")
        mapping_bytes = json.dumps(path_mapping or {}).encode("utf-8")

        ptr = func(
            ctypes.cast(sources_bytes, ctypes.c_void_p),
            ctypes.c_uint32(len(sources_bytes)),
            ctypes.cast(target_bytes, ctypes.c_void_p),
            ctypes.c_uint32(len(target_bytes)),
            ctypes.cast(mapping_bytes, ctypes.c_void_p),
            ctypes.c_uint32(len(mapping_bytes)),
        )
        if ptr is None:
            return CompileResult(success=False)

        result_json = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")

        dll.omni_free_string.argtypes = [ctypes.c_void_p]
        dll.omni_free_string.restype = None
        dll.omni_free_string(ptr)

        data = json.loads(result_json)
        return CompileResult.from_rust_response(data)

    @classmethod
    def analyse(cls, sources: dict[str, str],
                path_mapping: dict[str, str] | None = None) -> AnalyseResult:
        """Analyse .omni source files for diagnostics via Rust DLL.

        Calls omni_analyse C FFI which handles parsing and semantic analysis.

        Args:
            sources: {path: source_text}
            path_mapping: {import_path: resolved_path}

        Returns:
            AnalyseResult with success flag and diagnostics list.
        """
        dll = cls._load()
        func = dll.omni_analyse
        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        request = json.dumps({
            "sources": sources,
            "path_mapping": path_mapping or {},
        }).encode("utf-8")

        ptr = func(
            ctypes.cast(request, ctypes.c_void_p),
            ctypes.c_uint32(len(request)),
        )
        if ptr is None:
            return AnalyseResult(success=False)

        result_json = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")

        dll.omni_free_string.argtypes = [ctypes.c_void_p]
        dll.omni_free_string.restype = None
        dll.omni_free_string(ptr)

        data = json.loads(result_json)
        return AnalyseResult.from_rust_response(data)
