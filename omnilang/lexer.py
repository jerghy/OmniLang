"""Lexer wrapper for OmniLang — thin wrapper around lark.Lark.

Provides parser creation for both base OmniLang and OmniView (UI extension).
"""

from pathlib import Path
from lark import Lark


_GRAMMAR_DIR = Path(__file__).parent / "grammar"
_GRAMMAR_PATH = _GRAMMAR_DIR / "omnilang.lark"
_OMNIVIEW_GRAMMAR_PATH = Path(__file__).parent.parent / "omniview" / "grammar.lark"


def create_parser() -> Lark:
    """Create a Lark parser with the base OmniLang grammar (Earley mode)."""
    return Lark.open(
        str(_GRAMMAR_PATH),
        parser="earley",
        propagate_positions=True,
        maybe_placeholders=False,
    )


def create_omniview_parser() -> Lark:
    """Create a Lark parser with the OmniView UI grammar (Earley mode).

    Note: OmniView grammar includes its own expression/statement rules
    for backward compatibility. Phase A will progressively remove these
    duplicates and delegate to base OmniLang grammar.
    """
    if not _OMNIVIEW_GRAMMAR_PATH.exists():
        return create_parser()
    return Lark.open(
        str(_OMNIVIEW_GRAMMAR_PATH),
        parser="earley",
        propagate_positions=True,
        maybe_placeholders=False,
    )


def tokenize(source: str) -> list:
    """Tokenize OmniLang source code. Returns list of tokens."""
    parser = create_parser()
    return list(parser.lex(source))
