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

    # ── Private ────────────────────────────────────────

    @classmethod
    def _try_dll(cls, source: str) -> dict | None:
        """Try compiling via ctypes DLL call."""
        # TODO: Implement when Rust DLL is built (Phase 1)
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
        return None

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
