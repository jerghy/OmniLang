"""CLI entry point for the OmniLang compiler.

Commands:
    omnic build [files...] [--target TARGET] [--output DIR]   Compile .omni to target
    omnic analyse [files...] [--format json]                  Static analysis
    omnic run <file.omni> [--target TARGET]                   Compile and run
    omnic create <name> --type logic|view                     Scaffold a project
    omnic migrate [--file FILE] [--dir DIR]                   Convert .omni.json to annotated .omni files
    omnic serve --dir DIR [--port PORT]                       Visual editor server
    omnic watch <file.omni> [--target TARGET] [--output DIR]  Watch and auto-rebuild

All compilation routes through the Rust compiler via omnilang.api.compile_omni_multi().
"""

from __future__ import annotations

import argparse
import os
import sys

from omnilang.errors import OmniLangError, CompilerUnavailable, AnalyseError, InputError, OmniError, CompileError
from omnilang.rust_bridge import RustBridge


# ── Constants ──────────────────────────────────────────────

ALL_TARGETS = ["python", "js", "ts", "rust", "dart", "react", "vue", "dioxus", "flutter", "omniview"]

TARGET_EXTENSIONS: dict[str, str] = {
    "rust": ".rs",
    "dart": ".dart",
    "js": ".js",
    "ts": ".ts",
    "python": ".py",
}


# ── Templates (per spec §3.5) ──────────────────────────────

LOGIC_TEMPLATE = """// {name} — OmniLang logic project
fn main() {{
    print("Hello from {name}!");
}}
"""

VIEW_TEMPLATE = """// {name} — OmniLang view project
import omniview;

component App {{
    view {{
        <div class: app>
            <h1>{name}</h1>
            <p>Welcome to your OmniView app!</p>
        </div>
    }}
}}
"""


# ════════════════════════════════════════════════════════════
# Helpers
# ════════════════════════════════════════════════════════════

def _load_project_config():
    """Load project config (.omnilang.toml) if available, else None."""
    try:
        from omnilang.config import load_config
        return load_config()
    except Exception:
        return None


def _collect_sources(paths: list[str]) -> dict[str, str]:
    """Walk file/directory paths, auto-discover .omni files.

    When paths is empty, auto-discovers src/**/*.omni then ./*.omni.

    Returns {filepath: source_content} dict.

    Raises InputError if no sources found.
    """
    import glob as _glob

    source_files: list[str] = []

    if paths:
        for f in paths:
            if os.path.isdir(f):
                pattern = os.path.join(f, "**", "*.omni")
                source_files.extend(_glob.glob(pattern, recursive=True))
            else:
                if not os.path.exists(f):
                    raise InputError.file_not_found(f)
                source_files.append(f)
    else:
        # Auto-discover
        for d in ["src", "."]:
            pattern = os.path.join(d, "**", "*.omni")
            found = sorted(_glob.glob(pattern, recursive=True))
            if found:
                source_files = found
                break

    if not source_files:
        raise InputError([OmniError(
            severity="error", code="NO_SOURCES",
            message="No .omni files found. Specify file(s) or run from a project directory.",
        )])

    sources: dict[str, str] = {}
    for f in source_files:
        if not os.path.exists(f):
            raise InputError.file_not_found(f)
        with open(f, "r", encoding="utf-8") as fh:
            sources[f] = fh.read()

    return sources


def _detect_target(sources: dict[str, str], preferred: str | None = None, config=None) -> str:
    """Auto-detect target from source content, config, or --target flag.

    Priority: --target flag > config ui_target > auto-detect (UI or logic).
    """
    if preferred:
        return preferred

    if config is not None:
        ui_target = getattr(config, 'ui_target', None) or getattr(config, 'ui_target', '')
        if ui_target:
            return ui_target

    for src in sources.values():
        if "import omniview" in src or src.strip().startswith("component"):
            return "react"

    return "python"


# ════════════════════════════════════════════════════════════
# Build
# ════════════════════════════════════════════════════════════

def cmd_build(args):
    """Compile .omni source to target language(s).

    Usage:
      omnic build                          # auto-discover project
      omnic build app.omni                 # single file
      omnic build app.omni lib.omni        # multiple files
      omnic build src/                     # directory
    """
    config = _load_project_config()

    # Determine output dir from config or default
    output_dir = args.output
    if config is not None:
        try:
            output_dir = config.output.dir
        except AttributeError:
            pass

    # Collect sources
    sources = _collect_sources(args.files or [])

    # Determine target
    target = _detect_target(sources, args.target, config)

    # Compile
    from omnilang.api import compile_omni_multi
    result = compile_omni_multi(sources, target=target)

    if not result.success:
        raise CompileError([OmniError(
            severity="error", code="BUILD_FAILED",
            message="Compilation failed. Check diagnostics above for details.",
        )])

    # Write output files
    os.makedirs(output_dir, exist_ok=True)
    for filename, content in result.files.items():
        out_path = os.path.join(output_dir, filename)
        os.makedirs(os.path.dirname(out_path), exist_ok=True)
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  -> {out_path}")
    print(f"Compiled {len(result.files)} file(s) to {target} successfully.")


# ════════════════════════════════════════════════════════════
# Analyse (renamed from check)
# ════════════════════════════════════════════════════════════

def cmd_analyse(args):
    """Run static analysis on .omni source (via Rust compiler).

    Supports 3 modes (spec §3.2):
      omnic analyse              # auto-discover and analyse all files
      omnic analyse app.omni     # single file
      omnic analyse src/         # directory
    """
    # Collect sources (auto-discover, single file, or directory)
    sources = _collect_sources(args.files or [])

    from omnilang.api import _derive_path_mapping
    path_mapping = _derive_path_mapping(sources)
    try:
        result = RustBridge.analyse(sources, path_mapping)
    except CompilerUnavailable:
        raise

    if args.format == "json":
        import json
        diagnostics = [
            {
                "severity": d.severity,
                "code": d.code,
                "message": d.message,
                "file": d.file,
                "line": d.line,
                "col": d.col,
                "suggestion": d.suggestion,
            }
            for d in result.diagnostics
        ]
        print(json.dumps({"success": result.success, "diagnostics": diagnostics}))
        if not result.success:
            sys.exit(1)
        return

    # Terminal output
    for d in result.diagnostics:
        print(d.format())

    error_diags = [d for d in result.diagnostics if d.severity == "error"]
    if error_diags:
        print(f"\n{len(result.diagnostics)} diagnostic(s) — {len(error_diags)} error(s)")
        sys.exit(1)

    if result.diagnostics:
        # Warnings only
        print(f"\n{len(result.diagnostics)} warning(s)")
        return

    print("No issues found.")


# ════════════════════════════════════════════════════════════
# Run
# ════════════════════════════════════════════════════════════

def _write_standalone_omniview(output_dir: str, body: str, js: str, css: str) -> str:
    """Assemble omniview output into a runnable index.html."""
    index_path = os.path.join(output_dir, "index.html")
    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OmniView App</title>
    <style>{css}</style>
</head>
<body>
    <div id="root">{body}</div>
    <script>{js}</script>
</body>
</html>"""
    with open(index_path, "w", encoding="utf-8") as f:
        f.write(html)
    return index_path


def cmd_run(args):
    """Compile and run an OmniLang file.

    Output goes to build/debug/ per spec §3.2 and §3.3.
    Target is auto-detected when --target/-t is omitted.
    File auto-discovers src/main.omni when omitted (project root).
    """
    import subprocess

    filepath = args.file
    target = args.target

    # Auto-discover src/main.omni when no file specified (consistent with build/analyse)
    if filepath is None:
        default = os.path.join("src", "main.omni")
        if os.path.exists(default):
            filepath = default
        else:
            raise InputError([OmniError(
                severity="error", code="NO_ENTRY",
                message="No source file specified and src/main.omni not found. "
                        "Run 'omnic create <name>' to create a project, or specify a file.",
            )])

    if not os.path.exists(filepath):
        raise InputError.file_not_found(filepath)

    # Collect ALL project sources (not just the entry file) so cross-file
    # imports like `import { X } from "./components/X"` resolve correctly.
    sources = _collect_sources([])

    # Auto-detect target if not specified
    if target is None:
        target = _detect_target(sources)
        # View frameworks can't be run directly; run as omniview (HTML+JS+CSS in browser)
        if target in ("react", "vue", "dioxus", "flutter"):
            target = "omniview"

    from omnilang.api import compile_omni_multi
    result = compile_omni_multi(sources, target=target)

    if not result.success:
        raise CompileError([OmniError(
            severity="error", code="RUN_FAILED",
            message="Compilation failed. Check diagnostics above for details.",
        )])

    # Output to build/debug/ per spec §3.2 / §3.3
    output_dir = os.path.join("build", "debug")
    os.makedirs(output_dir, exist_ok=True)

    # Write all output files
    for filename, content in result.files.items():
        out_path = os.path.join(output_dir, filename)
        os.makedirs(os.path.dirname(out_path), exist_ok=True)
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(content)

    # OmniView: assemble standalone HTML and open in browser
    if target == "omniview":
        # Codegen produces body fragment + JS + CSS as separate files.
        # Assemble them into a single self-contained index.html.
        body = result.files.get("body.html", "") or result.files.get("index.html", "")
        js = result.files.get("index.js", "")
        css = result.files.get("index.css", "")
        index_path = _write_standalone_omniview(output_dir, body, js, css)

        import webbrowser
        webbrowser.open(f"file://{os.path.abspath(index_path)}")
        print(f"Opened {os.path.abspath(index_path)} in browser")
        return

    # Locate the first output file to execute
    first_name = next(iter(result.files.keys()), None)
    if not first_name:
        print("No output generated.", file=sys.stderr)
        sys.exit(1)

    out_path = os.path.join(output_dir, first_name)

    # Execute based on target
    if target == "python":
        subprocess.run(["python", out_path])
    elif target == "js":
        subprocess.run(["node", out_path])
    elif target == "ts":
        subprocess.run(["npx", "ts-node", out_path])
    elif target == "dart":
        subprocess.run(["dart", "run", out_path])
    elif target == "rust":
        import shutil
        rust_dir = os.path.join(output_dir, "rust_proj")
        src_dir = os.path.join(rust_dir, "src")
        os.makedirs(src_dir, exist_ok=True)
        shutil.copy(out_path, os.path.join(src_dir, "main.rs"))
        cargo_toml = os.path.join(rust_dir, "Cargo.toml")
        with open(cargo_toml, "w", encoding="utf-8") as f:
            f.write('[package]\nname = "omni_run"\nversion = "0.1.0"\nedition = "2021"\n')
        subprocess.run(["cargo", "run", "--quiet"], cwd=rust_dir)
    else:
        print(f"Don't know how to run target: {target}", file=sys.stderr)
        sys.exit(1)


# ════════════════════════════════════════════════════════════
# Create (scaffold project)
# ════════════════════════════════════════════════════════════

def cmd_create(args):
    """Scaffold a new OmniLang project with .omnilang.toml and src/main.omni.

    Usage: omnic create <name> --type logic|view
    """
    project_name = args.name
    project_type = args.type
    project_dir = os.path.abspath(project_name)

    if os.path.exists(project_dir):
        raise InputError([OmniError(
            severity="error", code="DIR_EXISTS",
            message=f"Directory '{project_name}' already exists.",
        )])

    os.makedirs(project_dir, exist_ok=True)

    # Create .omnilang.toml
    config_path = os.path.join(project_dir, ".omnilang.toml")
    with open(config_path, "w", encoding="utf-8") as f:
        f.write(f"""[project]
name = "{project_name}"
version = "0.1.0"

[source]
dir = "src"

[output]
dir = "build"
""")
    print(f"  Config: {config_path}")

    # Create src directory
    src_dir = os.path.join(project_dir, "src")
    os.makedirs(src_dir, exist_ok=True)

    # Create main.omni from template
    main_path = os.path.join(src_dir, "main.omni")
    template = LOGIC_TEMPLATE if project_type == "logic" else VIEW_TEMPLATE
    with open(main_path, "w", encoding="utf-8") as f:
        f.write(template.format(name=project_name))
    print(f"  Source: {main_path}")

    print(f"\nCreated {project_type} project '{project_name}' at {project_dir}")
    print(f"  cd {project_name}")
    print(f"  omnic run src/main.omni")


# ════════════════════════════════════════════════════════════
# Migrate (annotation migration)
# ════════════════════════════════════════════════════════════

def cmd_migrate(args):
    """Convert .omni.json to annotated .omni files.

    Usage:
      omnic migrate                         # default: project.omni.json in current dir
      omnic migrate --file custom.omni.json # specific file
      omnic migrate --dir ./my-project      # project directory
    """
    from pathlib import Path
    from editor.server.annotation.migrate import migrate_omni_json

    project_dir = Path(args.dir or ".")
    json_file = Path(args.file or "project.omni.json")

    if not json_file.is_absolute():
        json_file = project_dir / json_file

    migrate_omni_json(json_file, project_dir)


# ════════════════════════════════════════════════════════════
# Serve (visual editor)
# ════════════════════════════════════════════════════════════

def cmd_serve(args):
    """Start the visual editor server.

    Delegates to editor.server.main.cmd_serve which handles FastAPI + Vite.
    """
    from editor.server.main import cmd_serve as _serve
    _serve(args)


# ════════════════════════════════════════════════════════════
# Watch
# ════════════════════════════════════════════════════════════

def cmd_watch(args):
    """Watch .omni files and auto-rebuild on changes.

    Polls file mtime every 0.5s, recompiles via RustBridge on change,
    reports timestamped build results.
    """
    import time

    filepath = args.file
    target = args.target
    output_dir = args.output

    if not os.path.exists(filepath):
        raise InputError.file_not_found(filepath)

    print(f"Watching {filepath} (target={target}, output={output_dir})")
    print("Press Ctrl+C to stop.")
    print()

    last_mtime = os.path.getmtime(filepath)
    build_count = 0

    from omnilang.api import compile_omni_multi
    from omnilang.errors import OmniLangError

    try:
        while True:
            time.sleep(0.5)
            try:
                current_mtime = os.path.getmtime(filepath)
            except OSError:
                continue

            if current_mtime == last_mtime:
                continue
            last_mtime = current_mtime
            build_count += 1
            timestamp = time.strftime("%H:%M:%S")

            try:
                with open(filepath, "r", encoding="utf-8") as f:
                    source = f.read()
            except Exception as e:
                print(f"[{timestamp}] #{build_count} Read error: {e}")
                continue

            try:
                result = compile_omni_multi({filepath: source}, target=target)

                os.makedirs(output_dir, exist_ok=True)
                for filename, content in result.files.items():
                    out_path = os.path.join(output_dir, filename)
                    os.makedirs(os.path.dirname(out_path), exist_ok=True)
                    with open(out_path, "w", encoding="utf-8") as f:
                        f.write(content)

                file_count = len(result.files)
                print(f"[{timestamp}] #{build_count} Build succeeded — {file_count} file(s) -> {output_dir}/")
            except OmniLangError as e:
                lines = e.format().strip()
                if lines:
                    for line in lines.split("\n"):
                        print(f"[{timestamp}] #{build_count} {line}")
                else:
                    print(f"[{timestamp}] #{build_count} Build failed")
            except Exception as e:
                print(f"[{timestamp}] #{build_count} Error: {e}")
    except KeyboardInterrupt:
        print(f"\nStopped. ({build_count} build(s))")


# ════════════════════════════════════════════════════════════
# Main
# ════════════════════════════════════════════════════════════

def main():
    """Entry point with structured error handling."""
    try:
        _main()
    except OmniLangError as e:
        print(e.format(), file=sys.stderr)
        sys.exit(e.exit_code)
    except Exception as e:
        print(f"Internal error: {e}", file=sys.stderr)
        print("Please report this at https://github.com/omnilang/omnilang/issues", file=sys.stderr)
        sys.exit(99)


def _main():
    """Argument parsing and command dispatch."""
    parser = argparse.ArgumentParser(
        prog="omnic",
        description="OmniLang compiler — transpile .omni to Python, JS, TS, Rust, Dart, React, Vue, and more",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # ── build ──
    bp = subparsers.add_parser("build", help="Compile .omni source file(s) to target language(s)")
    bp.add_argument(
        "files", nargs="*",
        help="Source file(s) (.omni) or directory (default: auto-discover src/)",
    )
    bp.add_argument("--target", "-t", choices=ALL_TARGETS, help="Target language/framework")
    bp.add_argument("--output", "-o", default="./build", help="Output directory (default: ./build)")

    # ── analyse ──
    ap = subparsers.add_parser("analyse", help="Run static analysis on .omni source")
    ap.add_argument(
        "files", nargs="*",
        help="Source file(s) (.omni) or directory (default: auto-discover src/)",
    )
    ap.add_argument("--format", choices=["terminal", "json"], default="terminal", help="Output format")

    # ── run ──
    rp = subparsers.add_parser("run", help="Compile and run an OmniLang file")
    rp.add_argument(
        "file", nargs="?", default=None,
        help="Source file (.omni) — defaults to src/main.omni when in project root",
    )
    rp.add_argument("--target", "-t", choices=ALL_TARGETS, help="Target (auto-detected if omitted)")

    # ── create ──
    cp = subparsers.add_parser("create", help="Scaffold a new OmniLang project")
    cp.add_argument("name", help="Project name")
    cp.add_argument("--type", "-t", choices=["logic", "view"], required=True,
                    help="Project type (required): logic or view")

    # ── migrate ──
    mp = subparsers.add_parser("migrate", help="Convert .omni.json to annotated .omni files")
    mp.add_argument("--file", default="project.omni.json", help=".omni.json file path (default: project.omni.json)")
    mp.add_argument("--dir", default=".", help="Project directory (default: current dir)")

    # ── serve ──
    sp = subparsers.add_parser("serve", help="Start the visual editor server")
    sp.add_argument("--dir", required=True, help="Project directory to serve")
    sp.add_argument("--port", type=int, default=5173, help="Server port (default: 5173)")
    sp.add_argument("--vite-port", type=int, default=5174, help="Vite dev server port (default: 5174)")
    sp.add_argument("--dev", action="store_true", help="Force Vite dev mode")
    sp.add_argument("--no-frontend", action="store_true", help="Skip auto-starting Vite dev server")

    # ── watch ──
    wp = subparsers.add_parser("watch", help="Watch .omni files and auto-rebuild on changes")
    wp.add_argument("file", help="Source file (.omni)")
    wp.add_argument("--target", "-t", choices=ALL_TARGETS, default="react", help="Target (default: react)")
    wp.add_argument("--output", "-o", default="./build", help="Output directory (default: ./build)")

    args = parser.parse_args()

    if args.command == "build":
        cmd_build(args)
    elif args.command == "analyse":
        cmd_analyse(args)
    elif args.command == "run":
        cmd_run(args)
    elif args.command == "create":
        cmd_create(args)
    elif args.command == "migrate":
        cmd_migrate(args)
    elif args.command == "serve":
        cmd_serve(args)
    elif args.command == "watch":
        cmd_watch(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
