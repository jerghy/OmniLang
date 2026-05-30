"""Code generators for OmniLang target languages."""

from omnilang.ast_nodes import Program
from omnilang.codegen.python import PythonGenerator
from omnilang.codegen.javascript import JavaScriptGenerator
from omnilang.codegen.dart import DartGenerator
from omnilang.codegen.rust import RustGenerator


def generate(program: Program, target: str, rename_map: dict, type_map: dict) -> str:
    """Generate target language code from an analyzed OmniLang AST."""
    generators = {
        "rust": RustGenerator(),
        "dart": DartGenerator(),
        "js": JavaScriptGenerator(),
        "python": PythonGenerator(),
    }
    gen = generators.get(target)
    if gen is None:
        raise ValueError(f"Unknown target: {target}")
    return gen.generate(program, rename_map, type_map)
