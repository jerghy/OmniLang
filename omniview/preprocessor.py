"""Brace-balanced preprocessor for OmniView .omni source files.

Extracts statement blocks (fn bodies, action bodies, lifecycle hook bodies)
and replaces them with placeholder tokens. The OmniView Lark grammar then
only sees placeholders, not the raw statement text.

Architecture:
    .omni source → Preprocessor → processed source + block map
                                   │
                                   ├── Lark Parser (UI structure only)
                                   └── OmniLang Bridge (parses extracted blocks)
"""

from __future__ import annotations

from dataclasses import dataclass, field
import re
from typing import Callable


# ── Placeholder format ────────────────────────────────────

_PLACEHOLDER_RE = re.compile(r"__OMNI_(BLOCK|EXPR)_(\d+)__")


@dataclass
class BlockSlot:
    """Records an extracted statement block."""
    slot_id: str              # e.g. "__OMNI_BLOCK_0__"
    original_text: str        # Raw source text (without outer braces)
    omni_line: int            # 1-based line in .omni source
    omni_col: int             # 1-based column in .omni source
    context: str              # "fn_body" | "action_body" | "lifecycle_body" | "event_handler"


@dataclass
class PreprocessorOutput:
    """Output of the preprocessor."""
    processed_source: str              # Source with blocks replaced by placeholders
    blocks: dict[str, BlockSlot]       # placeholder_id → BlockSlot
    placeholder_count: int = 0


# ── Context patterns ─────────────────────────────────────

# Patterns that indicate a following { is a statement block
# NOTE: import { ... } from "..." is NOT matched by these patterns
# (none start with "import"), so cross-file imports pass through safely.
_BLOCK_CONTEXT_PATTERNS = [
    # fn name(params) -> type {     or   fn name(params) {
    (re.compile(r'\bfn\s+\w+\s*\([^)]*\)\s*(?:->\s*[\w<>|]+\s*)?\{'), 'fn_body'),
    # action name(params) -> type {  or   action name(params) {
    (re.compile(r'\baction\s+\w+\s*\([^)]*\)\s*(?:->\s*[\w<>|]+\s*)?\{'), 'action_body'),
    # async action name(params) -> type {
    (re.compile(r'\basync\s+action\s+\w+\s*\([^)]*\)\s*(?:->\s*[\w<>|]+\s*)?\{'), 'action_body'),
    # fn (params) {     inline event handler (no function name)
    (re.compile(r'\bfn\s*\(\s*[^)]*\)\s*\{'), 'event_handler'),
    # @mount {
    (re.compile(r'@mount\s*\{'), 'lifecycle_body'),
    # @unmount {
    (re.compile(r'@unmount\s*\{'), 'lifecycle_body'),
    # @effect(name) {
    (re.compile(r'@effect\s*\(\s*\w+\s*\)\s*\{'), 'lifecycle_body'),
]


def _find_blocks(source: str) -> list[BlockSlot]:
    """Find all statement blocks in source that should be delegated to OmniLang.

    Scans for fn bodies, action bodies, and lifecycle hook bodies.
    Uses brace balancing to correctly handle nested {} within blocks.

    Returns:
        List of BlockSlot objects sorted by their position in source.
    """
    blocks: list[BlockSlot] = []
    lines = source.split('\n')

    for pattern, context in _BLOCK_CONTEXT_PATTERNS:
        for match in pattern.finditer(source):
            # Find the opening brace position
            brace_pos = match.end() - 1  # pattern ends with '{'
            if brace_pos >= len(source) or source[brace_pos] != '{':
                continue

            # Extract block content using brace balancing
            block_text, end_pos = _extract_balanced_braces(source, brace_pos)
            if block_text is None:
                continue

            # Calculate line/col in original source
            line_num = source[:brace_pos].count('\n') + 1
            last_nl = source[:brace_pos].rfind('\n')
            col_num = brace_pos - last_nl if last_nl >= 0 else brace_pos + 1

            blocks.append(BlockSlot(
                slot_id="",  # filled in later
                original_text=block_text,
                omni_line=line_num,
                omni_col=col_num + 1,  # +1 because col is 1-based
                context=context,
            ))

    # Sort by position, remove duplicates (same brace_pos)
    blocks.sort(key=lambda b: (b.omni_line, b.omni_col))
    seen_positions = set()
    unique_blocks = []
    for b in blocks:
        pos_key = (b.omni_line, b.omni_col)
        if pos_key not in seen_positions:
            seen_positions.add(pos_key)
            b.slot_id = f"__OMNI_BLOCK_{len(unique_blocks)}__"
            unique_blocks.append(b)

    return unique_blocks


def _extract_balanced_braces(source: str, open_pos: int) -> tuple[str | None, int]:
    """Extract content between balanced braces starting at open_pos.

    Handles:
    - Nested braces
    - String literals (don't count braces inside strings)
    - Single-line comments (// ... \\n)
    - Block comments (/* ... */)

    Args:
        source: The full source text
        open_pos: Position of the opening '{'

    Returns:
        (inner_text, closing_brace_pos) or (None, -1) if braces are unbalanced
    """
    depth = 1
    i = open_pos + 1  # start after the opening brace
    in_string = False
    in_char = False
    in_line_comment = False
    in_block_comment = False
    escape_next = False

    while i < len(source) and depth > 0:
        ch = source[i]

        if escape_next:
            escape_next = False
            i += 1
            continue

        if in_string:
            if ch == '\\':
                escape_next = True
            elif ch == '"':
                in_string = False
            i += 1
            continue

        if in_char:
            if ch == '\\':
                escape_next = True
            elif ch == "'":
                in_char = False
            i += 1
            continue

        if in_line_comment:
            if ch == '\n':
                in_line_comment = False
            i += 1
            continue

        if in_block_comment:
            if ch == '*' and i + 1 < len(source) and source[i + 1] == '/':
                in_block_comment = False
                i += 2
                continue
            i += 1
            continue

        # Check for comment starts
        if ch == '/' and i + 1 < len(source):
            if source[i + 1] == '/':
                in_line_comment = True
                i += 2
                continue
            elif source[i + 1] == '*':
                in_block_comment = True
                i += 2
                continue

        # String start
        if ch == '"':
            in_string = True
        elif ch == "'":
            in_char = True
        elif ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return source[open_pos + 1:i], i

        i += 1

    if depth > 0:
        return None, -1  # Unmatched braces

    return source[open_pos + 1:i], i


def _strip_comments(source: str) -> str:
    """Strip single-line comments (//) and block comments (/* */) from source.

    Preserves string literals and doesn't strip comments inside them.
    NOTE: We intentionally do NOT track single-quote char literals.
    Single quotes in .omni source are typically apostrophes in text content
    (e.g., "Rust's async runtime"), not character literal delimiters.
    Tracking them would cause false in-char-literal state that prevents
    comment detection for the rest of the file.
    """
    result = []
    i = 0
    in_string = False
    escape_next = False
    while i < len(source):
        ch = source[i]

        if escape_next:
            escape_next = False
            result.append(ch)
            i += 1
            continue

        if in_string:
            result.append(ch)
            if ch == '\\':
                escape_next = True
            elif ch == '"':
                in_string = False
            i += 1
            continue

        # String start
        if ch == '"':
            in_string = True
            result.append(ch)
            i += 1
            continue

        # JSX-style comment {/* ... */} — strip entire construct
        # Must check BEFORE block comment and BEFORE appending '{'
        if ch == '{' and i + 2 < len(source) and source[i + 1] == '/' and source[i + 2] == '*':
            i += 3  # skip {/*
            while i + 1 < len(source):
                if source[i] == '*' and source[i + 1] == '/':
                    i += 2  # skip */
                    # consume optional closing '}'
                    if i < len(source) and source[i] == '}':
                        i += 1
                    break
                i += 1
            continue

        # Single-line comment
        if ch == '/' and i + 1 < len(source) and source[i + 1] == '/':
            # Skip until end of line
            while i < len(source) and source[i] != '\n':
                i += 1
            if i < len(source):
                result.append('\n')
                i += 1
            continue

        # Block comment
        if ch == '/' and i + 1 < len(source) and source[i + 1] == '*':
            i += 2  # skip /*
            while i + 1 < len(source):
                if source[i] == '*' and source[i + 1] == '/':
                    i += 2
                    break
                i += 1
            continue

        result.append(ch)
        i += 1

    return ''.join(result)


def preprocess(source: str) -> PreprocessorOutput:
    """Preprocess .omni source: extract statement blocks, replace with placeholders.

    This is the main entry point. Call before Lark parsing.

    Args:
        source: Raw .omni source text

    Returns:
        PreprocessorOutput with processed source and block mapping
    """
    # Strip comments first — they should never appear in output
    source = _strip_comments(source)
    blocks = _find_blocks(source)

    if not blocks:
        return PreprocessorOutput(
            processed_source=source,
            blocks={},
        )

    # Replace blocks from end to start (to preserve positions)
    result = source
    for block in reversed(blocks):
        # Find the original brace position
        pattern = _BLOCK_CONTEXT_PATTERNS[0][0]  # any pattern works, we know the position
        # Find the { at this position
        line_start = 0
        for _ in range(block.omni_line - 1):
            line_start = result.index('\n', line_start) + 1 if '\n' in result[line_start:] else len(result)
        col_offset = block.omni_col - 1
        brace_pos = line_start + col_offset

        # Use brace balancing to find the closing brace
        _, close_pos = _extract_balanced_braces(result, brace_pos)
        if close_pos < 0:
            continue

        # Replace the entire { ... } block with the placeholder
        placeholder = block.slot_id
        result = result[:brace_pos + 1] + placeholder + result[close_pos:]

    # Build block map
    block_map = {b.slot_id: b for b in blocks}

    return PreprocessorOutput(
        processed_source=result,
        blocks=block_map,
        placeholder_count=len(blocks),
    )


def is_placeholder(text: str) -> bool:
    """Check if a string is an OmniView placeholder token."""
    return bool(_PLACEHOLDER_RE.match(text))


def parse_placeholder_id(text: str) -> tuple[str, int] | None:
    """Extract placeholder type and index from a placeholder string.

    Returns:
        (type, index) e.g. ("BLOCK", 0), or None
    """
    m = _PLACEHOLDER_RE.match(text)
    if m:
        return (m.group(1), int(m.group(2)))
    return None
