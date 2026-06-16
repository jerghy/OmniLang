"""Build script for multi-file OmniLang project → OmniView standalone HTML.

Usage: python build.py
Reads src/*.omni and all imports, compiles via Rust DLL, assembles standalone HTML.
"""
import json
import os
import re
import sys
from pathlib import Path

# Ensure we can import omnilang
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from omnilang.rust_bridge import RustBridge


def collect_sources(entry: Path) -> tuple[dict[str, str], dict[str, str]]:
    """Walk import graph and collect all .omni source files.

    Returns:
        sources: {absolute_path: source_text}
        path_mapping: {import_path_as_written: resolved_absolute_path}
    """
    sources: dict[str, str] = {}
    path_mapping: dict[str, str] = {}
    src_dir = entry.parent

    # Regex to find imports: import { ... } from "..."
    import_re = re.compile(r'import\s+\{[^}]*\}\s+from\s+"([^"]+)"')

    pending = [str(entry.resolve()).replace("\\", "/")]
    while pending:
        filepath = pending.pop()
        if filepath in sources:
            continue

        with open(filepath, "r", encoding="utf-8") as f:
            source = f.read()
        sources[filepath] = source

        # Find local imports
        for match in import_re.finditer(source):
            import_path = match.group(1)
            if import_path.startswith("."):
                # Resolve relative import path
                current_dir = Path(filepath).parent
                resolved = str((current_dir / import_path).resolve()).replace("\\", "/")
                path_mapping[import_path] = resolved
                if resolved not in sources:
                    pending.append(resolved)

    return sources, path_mapping


def build(entry: Path, out_dir: Path) -> bool:
    """Build multi-file OmniLang project to standalone HTML."""
    print(f"Collecting sources from {entry}...")
    sources, path_mapping = collect_sources(entry)
    print(f"  Found {len(sources)} source file(s):")
    for p in sources:
        print(f"    - {p}")

    print("Compiling with multi-file support...")
    # Ensure DLL is loaded (is_available has a path bug with triple parent)
    dll = RustBridge._load_dll()
    if dll is None:
        print("ERROR: Rust compiler DLL not found")
        return False
    result = RustBridge.compile_to_target(sources, "omniview", path_mapping)

    if result is None:
        print("ERROR: Compilation returned None")
        return False

    if not result.get("success"):
        errors = result.get("errors", [])
        print(f"ERROR: Compilation failed with {len(errors)} error(s):")
        for e in errors:
            print(f"  - {e}")
        return False

    files = result.get("files", {})
    print(f"  Generated {len(files)} output file(s)")

    out_dir.mkdir(parents=True, exist_ok=True)

    # Write raw output files
    for filename, content in files.items():
        out_path = out_dir / filename
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  Wrote {out_path} ({len(content)} bytes)")

    # Assemble standalone HTML
    body = files.get("index.html", "")
    js = files.get("index.js", "")
    css = files.get("index.css", "")

    html = f'''<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Todo App — OmniLang</title>
<style>
*,*::before,*::after{{box-sizing:border-box}}
body{{margin:0;padding:20px;font-family:"Inter",system-ui,-apple-system,sans-serif;background:#06060f;min-height:100vh}}
{css}
</style>
</head>
<body>
<div id="root"></div>
<script>
{js}
</script>
</body>
</html>'''

    html_path = out_dir / "index.html"
    with open(html_path, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"  Assembled standalone HTML: {html_path} ({len(html)} bytes)")

    return True


if __name__ == "__main__":
    project_dir = Path(__file__).parent
    entry = project_dir / "src" / "main.omni"
    out_dir = project_dir / "build"

    success = build(entry, out_dir)
    sys.exit(0 if success else 1)
