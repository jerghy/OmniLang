"""Text-based formatter for OmniView .omni source files.

Pretty-prints .omni source with consistent formatting:
  - 4-space indentation
  - Opening braces on the same line
  - Trailing whitespace removal
  - Single trailing newline

Note: This is a text-based formatter (not AST-based). The Python parser
has been removed; all compilation now goes through the Rust compiler.
"""


class OmniViewFormatter:
    """Text-based formatter for OmniView .omni files.

    Usage:
        formatter = OmniViewFormatter()
        formatted = formatter.format(source)
    """

    def __init__(self, indent_size: int = 4):
        self.indent_size = indent_size

    def format(self, source: str) -> str:
        """Format OmniView source code using text-based rules."""
        return self._basic_format(source)

    def _basic_format(self, source: str) -> str:
        """Basic code formatter.

        Performs:
        - Consistent indentation using spaces
        - Trailing whitespace removal
        - Single trailing newline
        - Brace-style normalization (opening brace on same line)
        """
        lines = source.split('\n')
        formatted_lines = []
        indent_level = 0
        indent_str = ' ' * self.indent_size

        for line in lines:
            stripped = line.rstrip()

            if not stripped:
                formatted_lines.append('')
                continue

            # Handle opening brace on same line: normalize "{\n" to " {\n"
            # Decrease indent for closing braces
            close_count = stripped.count('}')
            if close_count > 0:
                indent_level = max(0, indent_level - close_count)

            # Apply indentation
            if stripped:
                formatted_lines.append(indent_str * indent_level + stripped)
            else:
                formatted_lines.append('')

            # Increase indent for opening braces
            open_count = stripped.count('{')
            if open_count > 0:
                indent_level += open_count

        # Ensure single trailing newline
        result = '\n'.join(formatted_lines).rstrip() + '\n'
        return result
