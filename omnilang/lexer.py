"""Lexer wrapper for OmniLang — thin wrapper around lark.Lark."""

from pathlib import Path
from lark import Lark


_GRAMMAR_PATH = Path(__file__).parent / "grammar" / "omnilang.lark"
_GRAMMAR_V02_PATH = Path(__file__).parent / "grammar" / "omnilang_v0.2.lark"


def create_parser(use_v02: bool = True) -> Lark:
    """Create a Lark parser with the OmniLang grammar.

    Args:
        use_v02: If True (default), use the v0.2 grammar with Phase 1+ features.
                 If False, use the original v0.1 grammar (LALR mode).
    """
    grammar_path = str(_GRAMMAR_V02_PATH if use_v02 else _GRAMMAR_PATH)

    if use_v02:
        # v0.2: struct_lit + var_ref ambiguity requires Earley parser.
        # Earley handles the CNAME ambiguity natively; LALR state merging
        # creates unrecoverable conflicts in for-loop body contexts.
        # Performance: Earley parses ~1000 lines in <200ms (acceptable per plan).
        return Lark.open(
            grammar_path,
            parser="earley",
            propagate_positions=True,
            maybe_placeholders=False,
        )
    else:
        return Lark.open(
            grammar_path,
            parser="lalr",
            propagate_positions=True,
            maybe_placeholders=False,
        )


def tokenize(source: str) -> list:
    """Tokenize OmniLang source code. Returns list of tokens."""
    parser = create_parser()
    return list(parser.lex(source))
