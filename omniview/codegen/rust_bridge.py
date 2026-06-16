"""Rust bridge — call Rust compiler DLL/CLI for .omni -> IR compilation."""
from __future__ import annotations

import json
import subprocess
from pathlib import Path


class RustBridge:
    """Bridge to the Rust omni-compiler (DLL or CLI)."""

    _dll = None
    _availability_checked = False
    _available = False

    @classmethod
    def is_available(cls) -> bool:
        """Check if Rust compiler is available.

        Returns False gracefully — callers must handle Python fallback.
        """
        if cls._availability_checked:
            return cls._available

        # Option 1: Check for native DLL
        dll_paths = [
            Path(__file__).parent.parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
            Path("omni-compiler/target/release/omni_compiler.dll"),
        ]
        for p in dll_paths:
            if p.exists():
                cls._available = True
                cls._availability_checked = True
                return True

        # Option 2: Check for CLI
        try:
            result = subprocess.run(
                ["omni-compiler", "--version"],
                capture_output=True, timeout=5, text=True,
            )
            if result.returncode == 0:
                cls._available = True
                cls._availability_checked = True
                return True
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass

        cls._availability_checked = True
        cls._available = False
        return False

    @classmethod
    def compile_to_ir(cls, source: str) -> dict:
        """Compile .omni -> IR JSON via Rust.

        Returns: IRProgram dict
        Raises: RuntimeError if Rust compiler is not available
        """
        if not cls.is_available():
            raise RuntimeError("Rust compiler not available")

        # Try DLL first, fallback to CLI
        result = cls._try_dll(source) or cls._try_cli(source)
        if result is None:
            raise RuntimeError("Rust compilation failed — no output")

        if "_rust_errors" in result:
            return result  # Rust found errors; caller handles diagnostics

        return result

    @classmethod
    def compile_to_omniview(cls, source: str) -> dict:
        """Compile .omni -> OmniView JS/HTML/CSS + SourceMap via Rust.

        Returns: {success, files?, source_map?, errors?}
        """
        if not cls.is_available():
            raise RuntimeError("Rust compiler not available")

        result = cls._try_dll_omniview(source) or cls._try_cli_omniview(source)
        if result is None:
            raise RuntimeError("Rust OmniView compilation failed")
        return result

    @classmethod
    def validate_multi(cls, source_map: dict[str, str], path_mapping: dict[str, str]) -> dict | None:
        """Call omni_validate_multi via FFI for cross-file validation.

        Returns the result dict with 'success' and 'errors' keys, or None if unavailable.
        """
        if not cls.is_available():
            return None

        import json
        input_data = {
            'sources': source_map,
            'path_mapping': path_mapping,
        }
        input_json = json.dumps(input_data)
        result = cls._call_dll("omni_validate_multi", input_json)
        return result

    @classmethod
    def compile_to_target(cls, sources: dict[str, str], target: str,
                          path_mapping: dict[str, str] | None = None) -> dict | None:
        """Compile .omni source files to target code via Rust DLL.

        Calls omni_compile_multi_to_target C FFI which handles parsing,
        semantic analysis, and codegen entirely in Rust.

        Args:
            sources: {path: source_text}
            target: target name (python/js/rust/dart/omniview/...)
            path_mapping: {import_path: resolved_path}

        Returns:
            { success: bool, files: {path: content}, errors: [...] }
            or None if Rust compiler (or this specific function) is not available.
        """
        import ctypes

        dll = cls._load_dll()
        if dll is None:
            return None

        try:
            func = dll.omni_compile_multi_to_target
        except AttributeError:
            return None  # Function not exported yet — caller should fall through

        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        sources_bytes = json.dumps(sources).encode("utf-8")
        target_bytes = target.encode("utf-8")
        mapping_bytes = json.dumps(path_mapping or {}).encode("utf-8")

        try:
            ptr = func(
                ctypes.cast(sources_bytes, ctypes.c_void_p),
                ctypes.c_uint32(len(sources_bytes)),
                ctypes.cast(target_bytes, ctypes.c_void_p),
                ctypes.c_uint32(len(target_bytes)),
                ctypes.cast(mapping_bytes, ctypes.c_void_p),
                ctypes.c_uint32(len(mapping_bytes)),
            )
            if ptr is None:
                return {"success": False, "errors": ["Rust returned null"], "files": {}}

            result_json = ctypes.cast(ptr, ctypes.c_char_p).value
            if result_json:
                result_json = result_json.decode("utf-8")

            dll.omni_free_string.argtypes = [ctypes.c_void_p]
            dll.omni_free_string.restype = None
            dll.omni_free_string(ptr)

            return json.loads(result_json) if result_json else {
                "success": False, "errors": ["empty result"], "files": {}
            }
        except Exception as e:
            return {"success": False,
                    "errors": [f"Rust compile_to_target failed: {e}"],
                    "files": {}}

    # ── Private ────────────────────────────────────────

    @classmethod
    def _try_dll(cls, source: str) -> dict | None:
        """Try compiling via ctypes DLL call."""
        return cls._call_dll("omni_compile_to_ir", source)

    @classmethod
    def _call_dll(cls, func_name: str, source: str) -> dict | None:
        """Call a function in the Rust DLL via ctypes."""
        import ctypes

        dll = cls._load_dll()
        if dll is None:
            return None

        try:
            func = getattr(dll, func_name)
            func.argtypes = [ctypes.c_char_p]
            func.restype = ctypes.c_void_p

            source_bytes = source.encode("utf-8")
            ptr = func(source_bytes)
            if ptr is None:
                return None

            # Read the returned C string
            result_json = ctypes.cast(ptr, ctypes.c_char_p).value
            if result_json:
                result_json = result_json.decode("utf-8")

            # Free the C string
            dll.omni_free_string.argtypes = [ctypes.c_void_p]
            dll.omni_free_string.restype = None
            dll.omni_free_string(ptr)

            if result_json:
                parsed = json.loads(result_json)
                # Unwrap CompileResult: {success, ir?, errors[]} -> IRProgram dict
                # Note: "ir" is skip_serialized when None, so check for "success" too
                # OmniView results have "files" instead of "ir"
                if isinstance(parsed, dict) and ("ir" in parsed or "success" in parsed):
                    # OmniView result: {success, files, source_map, errors}
                    if "files" in parsed:
                        return parsed  # Return OmniViewResult as-is
                    # Return IR even if success=false (semantic warnings shouldn't block)
                    if parsed.get("ir") is not None:
                        result = parsed["ir"]
                        if parsed.get("errors"):
                            result["_rust_warnings"] = parsed["errors"]
                        return result
                    if parsed.get("errors"):
                        return {"_rust_errors": parsed["errors"]}
                return parsed
        except Exception:
            pass
        return None

    @classmethod
    def _load_dll(cls):
        """Load the omni_compiler DLL. Returns ctypes.CDLL or None."""
        if cls._dll is not None:
            return cls._dll

        import ctypes
        dll_paths = [
            str(Path(__file__).parent.parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll"),
            str(Path("omni-compiler/target/release/omni_compiler.dll")),
        ]
        for p in dll_paths:
            try:
                if Path(p).exists():
                    cls._dll = ctypes.CDLL(p)
                    return cls._dll
            except OSError:
                continue
        return None

    @classmethod
    def _try_cli(cls, source: str) -> dict | None:
        """Try compiling via subprocess CLI call."""
        try:
            result = subprocess.run(
                ["omni-compiler", "compile", "--to-ir", "--format", "json"],
                input=source, capture_output=True, timeout=30, text=True,
            )
            if result.returncode == 0:
                return json.loads(result.stdout)
            else:
                try:
                    errors = json.loads(result.stderr)
                    return {"_rust_errors": errors.get("errors", [])}
                except json.JSONDecodeError:
                    return {"_rust_errors": [{"message": result.stderr}]}
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        return None

    @classmethod
    def _try_dll_omniview(cls, source: str) -> dict | None:
        """Try OmniView compilation via DLL."""
        return cls._call_dll("omni_compile_to_omniview", source)

    @classmethod
    def _try_cli_omniview(cls, source: str) -> dict | None:
        """Try OmniView compilation via CLI."""
        try:
            result = subprocess.run(
                ["omni-compiler", "compile", "--to-omniview", "--format", "json"],
                input=source, capture_output=True, timeout=30, text=True,
            )
            if result.returncode == 0:
                return json.loads(result.stdout)
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        return None
