"""gen-extern: Generate Rust extern declarations from TypeScript .d.ts files.

Converts TypeScript declaration files to Rust extern crate / wasm-bindgen
declarations for use with Dioxus and other Rust frontend frameworks.

Usage:
    python -m omniview.gen_extern input.d.ts --output extern.rs
    omnic gen-extern input.d.ts --output extern.rs

Supported patterns:
    - export function name(params): ReturnType;
    - export interface Name { field: Type; }
    - export type Alias = Type;
    - export const name: Type;
"""

import re
import sys
import os
from dataclasses import dataclass, field
from pathlib import Path


# ═══════════════════════════════════════════════════════════
# TypeScript → Rust type mapping
# ═══════════════════════════════════════════════════════════

TS_TO_RUST_TYPE: dict[str, str] = {
    "string": "String",
    "number": "f64",
    "boolean": "bool",
    "void": "()",
    "any": "JsValue",
    "unknown": "JsValue",
    "never": "!",
    "object": "JsValue",
    "string[]": "Vec<String>",
    "number[]": "Vec<f64>",
    "boolean[]": "Vec<bool>",
    "any[]": "Vec<JsValue>",
    "Date": "String",  # JS Date → ISO string
    "Promise": "js_sys::Promise",
    "Array": "Vec",
    "Map": "std::collections::HashMap",
    "Set": "std::collections::HashSet",
}


def ts_type_to_rust(ts_type: str) -> str:
    """Convert a TypeScript type to Rust type."""
    ts_type = ts_type.strip()

    # Remove trailing semicolon
    ts_type = ts_type.rstrip(";")

    # Handle generics: Array<T> → Vec<T>
    generic_match = re.match(r'(\w+)<(.+)>$', ts_type)
    if generic_match:
        base = generic_match.group(1)
        inner = generic_match.group(2)
        if base == "Array":
            return f"Vec<{ts_type_to_rust(inner)}>"
        if base == "Promise":
            return f"js_sys::Promise"
        if base == "Map":
            key, val = inner.split(",", 1) if "," in inner else (inner, "any")
            return f"std::collections::HashMap<{ts_type_to_rust(key.strip())}, {ts_type_to_rust(val.strip())}>"
        return f"{base}<{ts_type_to_rust(inner)}>"

    # Handle unions: string | number → JsValue
    if "|" in ts_type:
        return "JsValue"

    # Handle optional: Type | null / Type | undefined → Option<Type>
    if ts_type.endswith(" | null") or ts_type.endswith(" | undefined"):
        inner = ts_type.split("|")[0].strip()
        inner_rust = ts_type_to_rust(inner)
        if inner_rust == "JsValue":
            return "JsValue"
        return f"Option<{inner_rust}>"

    # Direct type mapping
    if ts_type in TS_TO_RUST_TYPE:
        return TS_TO_RUST_TYPE[ts_type]

    # Preserve user-defined types
    return ts_type


# ═══════════════════════════════════════════════════════════
# Parser
# ═══════════════════════════════════════════════════════════

@dataclass
class FunctionDecl:
    name: str
    params: list[tuple[str, str]]  # (name, type)
    return_type: str


@dataclass
class InterfaceDecl:
    name: str
    fields: list[tuple[str, str]]  # (name, type)


@dataclass
class ConstDecl:
    name: str
    type: str


@dataclass
class TypeAlias:
    name: str
    target: str


@dataclass
class DtsFile:
    functions: list[FunctionDecl] = field(default_factory=list)
    interfaces: list[InterfaceDecl] = field(default_factory=list)
    consts: list[ConstDecl] = field(default_factory=list)
    type_aliases: list[TypeAlias] = field(default_factory=list)


def parse_dts(content: str) -> DtsFile:
    """Parse a .d.ts file content into structured declarations."""
    result = DtsFile()

    # Remove comments
    content = re.sub(r'//[^\n]*', '', content)
    content = re.sub(r'/\*[\s\S]*?\*/', '', content)

    # Match export function name(params): ReturnType;
    func_pattern = re.compile(
        r'export\s+function\s+(\w+)\s*\(([^)]*)\)\s*:\s*([^;]+);'
    )
    for match in func_pattern.finditer(content):
        name = match.group(1)
        params_str = match.group(2).strip()
        return_type = match.group(3).strip()

        params = []
        if params_str:
            for param in params_str.split(","):
                param = param.strip()
                if ":" in param:
                    pname, ptype = param.split(":", 1)
                    params.append((pname.strip(), ptype.strip()))
                else:
                    params.append((param, "any"))

        result.functions.append(FunctionDecl(
            name=name, params=params, return_type=return_type,
        ))

    # Match export interface Name { field: Type; ... }
    iface_pattern = re.compile(
        r'export\s+interface\s+(\w+)\s*\{([^}]*)\}'
    )
    for match in iface_pattern.finditer(content):
        name = match.group(1)
        body = match.group(2)
        fields = []
        for line in body.split(";"):
            line = line.strip()
            if not line:
                continue
            if ":" in line:
                fname, ftype = line.split(":", 1)
                fields.append((fname.strip(), ftype.strip()))

        result.interfaces.append(InterfaceDecl(name=name, fields=fields))

    # Match export const name: Type;
    const_pattern = re.compile(
        r'export\s+const\s+(\w+)\s*:\s*([^;]+);'
    )
    for match in const_pattern.finditer(content):
        result.consts.append(ConstDecl(
            name=match.group(1), type=match.group(2).strip(),
        ))

    # Match export type Alias = Type;
    type_pattern = re.compile(
        r'export\s+type\s+(\w+)\s*=\s*([^;]+);'
    )
    for match in type_pattern.finditer(content):
        result.type_aliases.append(TypeAlias(
            name=match.group(1), target=match.group(2).strip(),
        ))

    return result


# ═══════════════════════════════════════════════════════════
# Rust Code Generator
# ═══════════════════════════════════════════════════════════

def generate_rust(dts: DtsFile, module_name: str = "generated") -> str:
    """Generate Rust code from parsed .d.ts declarations."""
    L = []
    L.append("// Auto-generated by omnic gen-extern")
    L.append("// Source: .d.ts → Rust extern declarations")
    L.append("")
    L.append("use wasm_bindgen::prelude::*;")
    L.append("")
    L.append(f"#[wasm_bindgen(module = \"{module_name}\")]")
    L.append("extern \"C\" {")

    # Functions
    for func in dts.functions:
        params_rust = []
        for pname, ptype in func.params:
            rust_type = ts_type_to_rust(ptype)
            params_rust.append(f"{pname}: {rust_type}")

        return_rust = ts_type_to_rust(func.return_type)
        param_str = ", ".join(params_rust)
        L.append(f"    #[wasm_bindgen(js_name = \"{func.name}\")]")
        L.append(f"    pub fn {func.name}({param_str}) -> {return_rust};")
        L.append("")

    L.append("}")

    # Interfaces → Rust structs
    for iface in dts.interfaces:
        L.append("")
        L.append(f"#[derive(Clone, Debug, PartialEq)]")
        L.append(f"#[wasm_bindgen]")
        L.append(f"pub struct {iface.name} {{")
        for fname, ftype in iface.fields:
            rust_type = ts_type_to_rust(ftype)
            L.append(f"    pub {fname}: {rust_type},")
        L.append("}")

    # Type aliases → Rust type aliases
    for alias in dts.type_aliases:
        rust_target = ts_type_to_rust(alias.target)
        L.append("")
        L.append(f"pub type {alias.name} = {rust_target};")

    # Constants
    for const in dts.consts:
        rust_type = ts_type_to_rust(const.type)
        L.append("")
        L.append(f"#[wasm_bindgen]")
        L.append(f"extern \"C\" {{")
        L.append(f"    pub static {const.name}: {rust_type};")
        L.append(f"}}")

    return "\n".join(L) + "\n"


# ═══════════════════════════════════════════════════════════
# CLI Integration
# ═══════════════════════════════════════════════════════════

def gen_extern_command(args: list[str]) -> int:
    """CLI entry point: omnic gen-extern <file.d.ts> [--output <file.rs>]"""
    if not args:
        print("Usage: omnic gen-extern <file.d.ts> [--output <file.rs>] [--module <name>]", file=sys.stderr)
        return 1

    input_file = args[0]
    output_file = None
    module_name = "generated"

    i = 1
    while i < len(args):
        if args[i] == "--output" and i + 1 < len(args):
            output_file = args[i + 1]
            i += 2
        elif args[i] == "--module" and i + 1 < len(args):
            module_name = args[i + 1]
            i += 2
        else:
            i += 1

    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}", file=sys.stderr)
        return 1

    with open(input_file, "r", encoding="utf-8") as f:
        content = f.read()

    dts = parse_dts(content)
    rust_code = generate_rust(dts, module_name)

    if output_file:
        with open(output_file, "w", encoding="utf-8") as f:
            f.write(rust_code)
        print(f"Generated: {output_file}")
    else:
        print(rust_code)

    # Summary
    print(f"\n  Functions: {len(dts.functions)}")
    print(f"  Interfaces: {len(dts.interfaces)}")
    print(f"  Constants: {len(dts.consts)}")
    print(f"  Type aliases: {len(dts.type_aliases)}")

    return 0


if __name__ == "__main__":
    sys.exit(gen_extern_command(sys.argv[1:]))
