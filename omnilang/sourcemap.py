"""Source Map v3 encoder for OmniView generated code.

Produces standard Source Map v3 JSON files that map generated
TypeScript/JavaScript/Dart/Rust code back to .omni source positions.

Spec: https://sourcemaps.info/spec.html

Usage:
    mapper = SourceMapper(omni_filename="App.omni")
    mapper.add_mapping(generated_line=10, generated_col=0,
                       original_line=5, original_col=4,
                       symbol_name="count")
    mapper.add_mapping(generated_line=12, generated_col=2,
                       original_line=6, original_col=4)
    # ... generate code while tracking positions ...
    map_json = mapper.encode()  # JSON string
"""

from __future__ import annotations

import base64
import json
from dataclasses import dataclass, field
from typing import Any


# ═══════════════════════════════════════════════════════════
# VLQ (Variable-Length Quantity) encoding
# ═══════════════════════════════════════════════════════════

VLQ_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


def _encode_vlq(value: int) -> str:
    """Encode an integer as a VLQ (base64-vlq) string.

    Each VLQ digit uses 5 bits for the value and 1 bit for continuation.
    """
    if value < 0:
        value = ((-value) << 1) | 1  # Sign bit
    else:
        value = value << 1

    result = []
    while value > 0x1F:  # More than 5 bits
        result.append(VLQ_CHARS[(value & 0x1F) | 0x20])  # 0x20 = continuation bit
        value >>= 5
    result.append(VLQ_CHARS[value & 0x1F])
    return "".join(result)


# ═══════════════════════════════════════════════════════════
# Source Mapper
# ═══════════════════════════════════════════════════════════

@dataclass
class Mapping:
    """A single source mapping entry."""
    generated_line: int     # 0-based in generated output
    generated_col: int      # 0-based in generated output
    original_line: int      # 0-based in .omni source
    original_col: int       # 0-based in .omni source
    source_index: int = 0   # Index into sources array
    name_index: int | None = None  # Index into names array (optional)


@dataclass
class SourceMapper:
    """Accumulates source mappings and produces Source Map v3 output.

    Tracks line/column positions as code is generated. Call add_mapping()
    at each point where code maps back to .omni source.

    Args:
        omni_filename: The .omni source filename (relative path for the map)
        generated_filename: The generated output filename
    """

    omni_filename: str
    generated_filename: str = ""
    mappings: list[Mapping] = field(default_factory=list)
    _names: list[str] = field(default_factory=list)
    _name_indices: dict[str, int] = field(default_factory=dict)
    _sources: list[str] = field(default_factory=list)

    def __post_init__(self):
        if self.omni_filename:
            self._sources.append(self.omni_filename)

    def add_mapping(self, generated_line: int, generated_col: int,
                    original_line: int, original_col: int,
                    symbol_name: str | None = None):
        """Record a mapping from generated code position to .omni source position.

        All positions are 0-based.

        Args:
            generated_line: 0-based line in generated output
            generated_col: 0-based column in generated output
            original_line: 0-based line in .omni source
            original_col: 0-based column in .omni source
            symbol_name: Optional symbol name (variable, function, etc.)
        """
        name_idx = None
        if symbol_name:
            if symbol_name not in self._name_indices:
                self._name_indices[symbol_name] = len(self._names)
                self._names.append(symbol_name)
            name_idx = self._name_indices[symbol_name]

        self.mappings.append(Mapping(
            generated_line=generated_line,
            generated_col=generated_col,
            original_line=original_line,
            original_col=original_col,
            source_index=0,
            name_index=name_idx,
        ))

    def encode(self) -> str:
        """Encode all mappings to Source Map v3 JSON.

        Returns:
            JSON string conforming to Source Map v3 specification.
        """
        if not self.mappings:
            # Return minimal valid source map
            return json.dumps({
                "version": 3,
                "sources": self._sources,
                "names": self._names,
                "mappings": "",
                "file": self.generated_filename,
            })

        # Sort mappings by generated position
        sorted_mappings = sorted(self.mappings, key=lambda m: (m.generated_line, m.generated_col))

        # Encode mappings in VLQ format
        mappings_str = self._encode_mappings(sorted_mappings)

        result: dict[str, Any] = {
            "version": 3,
            "sources": self._sources,
            "names": self._names,
            "mappings": mappings_str,
        }
        if self.generated_filename:
            result["file"] = self.generated_filename
        if self.omni_filename:
            result["sourceRoot"] = ""

        return json.dumps(result, separators=(",", ":"))

    def _encode_mappings(self, mappings: list[Mapping]) -> str:
        """Encode a list of Mapping objects into VLQ segments.

        Each line's mappings are separated by ';'.
        Within a line, mappings are separated by ','.
        Each mapping uses relative positions (delta from previous).
        """
        lines: dict[int, list[Mapping]] = {}
        for m in mappings:
            if m.generated_line not in lines:
                lines[m.generated_line] = []
            lines[m.generated_line].append(m)

        # Previous values for delta encoding
        prev_gen_line = 0
        prev_gen_col = 0
        prev_src_idx = 0
        prev_src_line = 0
        prev_src_col = 0

        result_segments: list[str] = []

        max_line = max(lines.keys()) if lines else 0
        for line_num in range(max_line + 1):
            if line_num > 0:
                result_segments.append(";")

            if line_num not in lines:
                prev_gen_col = 0
                continue

            line_mappings = sorted(lines[line_num], key=lambda m: m.generated_col)
            line_parts: list[str] = []

            for m in line_mappings:
                # Delta encode
                gen_col_delta = m.generated_col - prev_gen_col if m.generated_line == prev_gen_line else m.generated_col
                src_idx_delta = m.source_index - prev_src_idx
                src_line_delta = m.original_line - prev_src_line
                src_col_delta = m.original_col - prev_src_col

                encoded = _encode_vlq(gen_col_delta)

                if src_idx_delta != 0 or src_line_delta != 0 or src_col_delta != 0:
                    encoded += _encode_vlq(src_idx_delta)
                    encoded += _encode_vlq(src_line_delta)
                    encoded += _encode_vlq(src_col_delta)

                    if m.name_index is not None:
                        # Name index delta (simplified: always relative to 0)
                        encoded += _encode_vlq(m.name_index)

                line_parts.append(encoded)

                # Update previous values
                prev_gen_line = m.generated_line
                prev_gen_col = m.generated_col
                prev_src_idx = m.source_index
                prev_src_line = m.original_line
                prev_src_col = m.original_col

            result_segments.append(",".join(line_parts))
            prev_gen_col = 0

        return "".join(result_segments)

    def write(self, output_path: str) -> None:
        """Write the source map to a .map file.

        Args:
            output_path: Path for the .map file (usually generated_file + ".map")
        """
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(self.encode())

    @classmethod
    def from_codegen(cls, omni_path: str, generated_path: str,
                     source_lines: dict[int, str] | None = None) -> "SourceMapper":
        """Create a SourceMapper for a codegen operation.

        Args:
            omni_path: Path to the .omni source file
            generated_path: Path for the generated output file
            source_lines: Optional dict mapping generated lines to omni lines
        """
        # Use relative paths for portability
        import os
        rel_source = os.path.basename(omni_path)
        rel_generated = os.path.basename(generated_path)

        mapper = cls(omni_filename=rel_source, generated_filename=rel_generated)
        return mapper


# ═══════════════════════════════════════════════════════════
# Line Tracker — tracks current position during code generation
# ═══════════════════════════════════════════════════════════

@dataclass
class LineTracker:
    """Tracks line/column position during code generation.

    Provides methods to emit code while tracking mappings back to .omni source.

    Usage:
        tracker = LineTracker(mapper, start_omni_line=5)
        tracker.emit("const [count, setCount] = useState(0);", omni_line=7)
        tracker.emit("")  # blank line, no mapping
        code, final_map = tracker.finish()
    """

    mapper: SourceMapper
    _lines: list[str] = field(default_factory=list)
    _current_line: int = 0
    _current_col: int = 0

    def emit(self, text: str, omni_line: int = 0, omni_col: int = 0,
             symbol: str | None = None):
        """Emit text at the current position, optionally with a source mapping.

        Args:
            text: The text to emit (a single line or partial line)
            omni_line: 1-based line in .omni source (0 = no mapping)
            omni_col: 1-based column in .omni source (0 = no mapping)
            symbol: Optional symbol name for the mapping
        """
        if not text:
            self._lines.append("")
            self._current_line += 1
            self._current_col = 0
            return

        # Handle multi-line text
        parts = text.split("\n")
        for i, part in enumerate(parts):
            if i > 0:
                self._lines.append("")
                self._current_line += 1
                self._current_col = 0

            if part:
                if omni_line > 0 and i == 0:
                    self.mapper.add_mapping(
                        generated_line=self._current_line,
                        generated_col=self._current_col,
                        original_line=omni_line - 1,  # 0-based
                        original_col=omni_col - 1,    # 0-based
                        symbol_name=symbol,
                    )
                self._current_col += len(part)

    def emit_line(self, text: str = "", omni_line: int = 0, omni_col: int = 0,
                  symbol: str | None = None):
        """Emit a complete line of code, advancing to the next line."""
        self.emit(text, omni_line, omni_col, symbol)
        self._lines.append("")
        self._current_line += 1
        self._current_col = 0

    def finish(self) -> tuple[str, str]:
        """Return the generated code and source map JSON.

        Returns:
            (generated_code, source_map_json)
        """
        code = "\n".join(self._lines)
        map_json = self.mapper.encode()
        return code, map_json
