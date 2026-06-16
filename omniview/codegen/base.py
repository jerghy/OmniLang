"""Base class for OmniView codegen backends.

Each target framework (React, Vue, Dioxus, Flutter) extends CodegenBase
and implements the three-stage pipeline:
  1. Pattern Matching  — recognize IR intent patterns
  2. Idiomatic Selection — pick framework-idiomatic implementation
  3. Code Expansion     — generate source code
"""

from dataclasses import dataclass, field


@dataclass
class CodegenResult:
    """Output from a codegen backend."""
    files: dict[str, str] = field(default_factory=dict)  # filename → source code
    warnings: list[str] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)

    @property
    def success(self) -> bool:
        return len(self.errors) == 0


class CodegenBase:
    """Base class for framework-specific code generators.

    Subclasses implement the three-stage pipeline:
      - _pattern_match(): identify IR patterns
      - _select_idiom(): choose the most idiomatic implementation
      - _expand(): generate target source code
    """

    # Target framework identifier
    target: str = "unknown"

    # Idiomaticity tier: L2 (idiomatic), L1 (readable), L0 (functional)
    tier: str = "L2"

    # Element tag → framework element mapping
    ELEMENT_MAP: dict[str, str] = {}

    # Semantic property → CSS/framework property mapping
    STYLE_MAP: dict[str, dict[str, str]] = {}

    def generate(self, ir) -> CodegenResult:
        """Run the full codegen pipeline on a component/store IR."""
        result = CodegenResult()

        try:
            # Stage 1: Pattern matching
            patterns = self._pattern_match(ir)

            # Stage 2: Idiomatic selection
            idioms = self._select_idiom(patterns)

            # Stage 3: Code expansion
            files = self._expand(ir, idioms)
            result.files = files
        except Exception as e:
            result.errors.append(f"Codegen failed: {e}")

        return result

    def _pattern_match(self, ir) -> dict:
        """Identify IR intent patterns. Override in subclasses."""
        return {}

    def _select_idiom(self, patterns: dict) -> dict:
        """Select framework-idiomatic implementations. Override in subclasses."""
        return {}

    def _expand(self, ir, idioms: dict) -> dict[str, str]:
        """Generate target source code. Override in subclasses."""
        return {}

    # ── Shared helpers ─────────────────────────────────

    @staticmethod
    def indent(code: str, level: int = 1, width: int = 4) -> str:
        """Indent each line of code by level * width spaces."""
        prefix = " " * (level * width)
        return "\n".join(prefix + line if line else "" for line in code.split("\n"))

    @staticmethod
    def line_comment(text: str, source_line: int | None = None) -> str:
        """Generate a source line number comment."""
        if source_line is not None:
            return f"/* .omni:{source_line} — {text} */"
        return f"/* {text} */"

    def map_element(self, tag: str) -> str:
        """Map an OmniView abstract element to a framework element."""
        return self.ELEMENT_MAP.get(tag, tag.lower())

    def map_style(self, prop: str, value: str) -> str | None:
        """Map a semantic style property to framework-specific output."""
        mapping = self.STYLE_MAP.get(prop, {})
        return mapping.get(value, mapping.get("_default"))


class _DictAccessor:
    """Wrap a dict for attribute-style access (minimal migration from dataclass->dict).

    Used by codegens that expect dataclass-style `.name` access but receive
    Rust-generated IR JSON dicts. All codegens should use `.get()` / `_get()`
    for compatibility; this accessor provides a safety net for direct access patterns.
    """
    _reserved = {'_data'}

    def __init__(self, data: dict):
        self.__dict__['_data'] = data

    def __getattr__(self, name):
        if name.startswith('_'):
            return self.__dict__.get(name)
        val = self.__dict__['_data'].get(name)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [_DictAccessor(item) if isinstance(item, dict) else item for item in val]
        return val

    def __setattr__(self, name, value):
        self.__dict__['_data'][name] = value

    def __contains__(self, key):
        return key in self.__dict__['_data']

    def get(self, key, default=None):
        val = self.__dict__['_data'].get(key, default)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [_DictAccessor(item) if isinstance(item, dict) else item for item in val]
        return val

    def __getitem__(self, key):
        return self.__getattr__(key)

    def __repr__(self):
        return f"_DictAccessor({self.__dict__['_data']!r})"


def ir_dict_to_accessor(d: dict) -> _DictAccessor:
    """Convert an IR dict to attribute-style accessor."""
    return _DictAccessor(d)
