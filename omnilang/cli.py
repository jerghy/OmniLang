"""CLI entry point for the OmniLang compiler.

Commands:
    omnic build <file.omni> [--target ...]  Compile .omni to target language(s)
    omnic check <file.omni>                Run static analysis (type + lint)
    omnic fix <file.omni>                  Auto-fix diagnostics
    omnic run <file.omni> [--target ...]   Compile and run
    omnic init [dir]                       Initialize a new project
    omnic fmt <file.omni>                  Format source code
    omnic serve --dir <dir>                Start the visual editor server
    omnic watch <file.omni>                Watch and auto-rebuild on changes

All compilation now routes through the Rust compiler via omnilang.api.compile_omni().
"""

import argparse
import sys
import os
import re

# ── Constants ──────────────────────────────────────────────

TARGET_EXTENSIONS = {
    "rust": ".rs",
    "dart": ".dart",
    "js": ".js",
    "python": ".py",
}

ALL_TARGETS = ["rust", "dart", "js", "python", "react", "vue", "dioxus", "flutter", "omniview"]


# ════════════════════════════════════════════════════════════
# Build
# ════════════════════════════════════════════════════════════

def cmd_build(args):
    """Compile .omni source to target language(s).

    Routes through omnilang.api.compile_omni() which handles:
    - Auto-detection of UI mode vs pure logic mode
    - Rust codegen (via RustBridge)
    """
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    # Load project config for output dir and aliases
    output_dir = args.output
    try:
        from omnilang.config import load_config
        cfg = load_config()
        if args.output == "./build":
            output_dir = cfg.output.dir if cfg.output.dir else "build"
    except ImportError:
        pass

    try:
        with open(args.file, "r", encoding="utf-8") as f:
            source = f.read()

        # Determine target(s)
        if args.target:
            targets = [args.target]
        else:
            # Auto-detect: UI mode defaults to react, pure logic to python
            targets = ["react"] if _detect_ui_mode(source) else ["python"]

        from omnilang.api import compile_omni

        os.makedirs(output_dir, exist_ok=True)

        for target in targets:
            result = compile_omni(source, filepath=args.file, target=target)
            if result.success:
                for filename, content in result.files.items():
                    out_path = os.path.join(output_dir, filename)
                    with open(out_path, "w", encoding="utf-8") as f:
                        f.write(content)
                    print(f"  -> {out_path}")
                print(f"Compiled to {target} successfully.")
            else:
                for err in result.errors:
                    sev = "Warning" if err.severity == "warning" else "Error"
                    loc = f"({err.file}:{err.line})" if err.file and err.file != "<input>" else ""
                    print(f"  {sev}: {err.message} {loc}".strip(), file=sys.stderr)
                if any(e.severity == "error" for e in result.errors):
                    sys.exit(1)

    except SystemExit:
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


# ════════════════════════════════════════════════════════════
# Check
# ════════════════════════════════════════════════════════════

def cmd_check(args):
    """Run static analysis on .omni source (via Rust compiler if available).

    Note: Python-based lint rules have been migrated to the Rust compiler.
    This command now delegates to the Rust compiler for all analysis.
    """
    if args.explain:
        cmd_explain(args.explain)
        return

    if not args.file or not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(args.file, "r", encoding="utf-8") as f:
            source = f.read()

        # Use Rust compiler for static analysis
        from omnilang.api import compile_omni
        result = compile_omni(source, filepath=args.file, target="python")

        diagnostics = [e for e in result.errors if e.severity == "error"]
        warnings = [e for e in result.errors if e.severity == "warning"]

        # Also try Rust-specific analysis if available
        from omnilang.rust_bridge import RustBridge
        if RustBridge.is_available():
            ir_result = RustBridge.compile_to_ir(source)
            if ir_result and isinstance(ir_result, dict):
                for warn in ir_result.get("_rust_warnings", []):
                    if isinstance(warn, dict):
                        warnings.append(type('Diag', (), {
                            'code': warn.get('code', 'RUST'),
                            'message': warn.get('message', str(warn)),
                            'line': warn.get('line', 1),
                            'file': args.file,
                        }))

        # Print results
        for d in diagnostics:
            loc = f"({d.file}:{d.line})" if d.file and d.file != "<input>" else ""
            print(f"  Error: {d.message} {loc}".strip(), file=sys.stderr)
        for w in warnings:
            loc = f"({args.file}:{getattr(w, 'line', 1)})" if hasattr(w, 'line') else ""
            print(f"  Warning: {getattr(w, 'message', str(w))} {loc}".strip())

        if not diagnostics and not warnings:
            print("No issues found.")
            sys.exit(0)
        elif diagnostics:
            print(f"\n{len(diagnostics)} error(s), {len(warnings)} warning(s)")
            sys.exit(1)
        else:
            print(f"\n{len(warnings)} warning(s)")
            sys.exit(0)

    except SystemExit:
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


def cmd_explain(code: str):
    """Show detailed explanation for a diagnostic code."""
    explain_dir = os.path.join(os.path.dirname(__file__), "analyze", "explain")
    index_path = os.path.join(explain_dir, "index.toml")

    if not os.path.exists(index_path):
        print(f"No explain index found at {index_path}")
        sys.exit(1)

    try:
        import tomllib
    except ImportError:
        import tomli as tomllib

    with open(index_path, "rb") as f:
        index = tomllib.load(f)

    entry = index.get(code.upper())
    if entry is None:
        print(f"No explain entry for {code}")
        sys.exit(1)

    print(f"=== {code}: {entry.get('title', '')} ===")
    print(f"Category: {entry.get('category', '')}")
    print(f"Severity: {entry.get('severity', '')}")
    print()
    print(entry.get('description', '').strip())
    print()
    print(f"Fix: {entry.get('fix', '')}")


# ════════════════════════════════════════════════════════════
# Fix
# ════════════════════════════════════════════════════════════

def cmd_fix(args):
    """Auto-fix diagnostics in a source file.

    WARNING: Python-based lint rules have been removed. The Rust compiler
    handles all static analysis. This command is preserved for compatibility
    and delegates to the Rust compiler.
    """
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    print("Note: Auto-fix is not available in Rust-only mode. Use omnic check to see diagnostics.")
    sys.exit(0)


# ════════════════════════════════════════════════════════════
# Run
# ════════════════════════════════════════════════════════════

def cmd_run(args):
    """Compile and immediately run an OmniLang file."""
    import subprocess
    import tempfile

    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    with open(args.file, "r", encoding="utf-8") as f:
        source = f.read()

    # Compile via Rust
    from omnilang.api import compile_omni
    result = compile_omni(source, filepath=args.file, target=args.target)

    if not result.success:
        for err in result.errors:
            print(f"  Error: {err.message}", file=sys.stderr)
        sys.exit(1)

    code = list(result.files.values())[0] if result.files else ""

    tmpdir = tempfile.mkdtemp(prefix="omni_run_")
    base = os.path.splitext(os.path.basename(args.file))[0]
    ext = TARGET_EXTENSIONS.get(args.target, ".txt")
    out = os.path.join(tmpdir, f"{base}{ext}")
    with open(out, "w", encoding="utf-8") as f:
        f.write(code)

    if args.target == "python":
        subprocess.run(["python", out], capture_output=False)
    elif args.target == "js":
        subprocess.run(["node", out], capture_output=False)
    elif args.target == "dart":
        subprocess.run(["dart", "run", out], capture_output=False)
    elif args.target == "rust":
        import shutil
        rust_dir = os.path.join(tmpdir, "rust_proj")
        os.makedirs(os.path.join(rust_dir, "src"), exist_ok=True)
        shutil.copy(out, os.path.join(rust_dir, "src", "main.rs"))
        with open(os.path.join(rust_dir, "Cargo.toml"), "w") as f:
            f.write('[package]\nname = "omni_run"\nversion = "0.1.0"\nedition = "2021"\n')
        subprocess.run(["cargo", "run", "--quiet"], cwd=rust_dir, capture_output=False)
    else:
        print(f"Unknown target: {args.target}", file=sys.stderr)
        sys.exit(1)


# ════════════════════════════════════════════════════════════
# Init
# ════════════════════════════════════════════════════════════

def cmd_init(args):
    """Initialize a new OmniLang project."""
    target_dir = args.dir or "."
    project_name = args.name or os.path.basename(os.path.abspath(target_dir))

    from omnilang.config import generate_config as gen_cfg, load_config, validate_config

    try:
        config_path = gen_cfg(target_dir, project_name)
        config = load_config(config_path)
        errors = validate_config(config)

        # Create source directory
        src_dir = os.path.join(target_dir, config.source_dir)
        os.makedirs(src_dir, exist_ok=True)

        # Create a hello-world source file
        hello_path = os.path.join(src_dir, "main.omni")
        if not os.path.exists(hello_path):
            with open(hello_path, "w", encoding="utf-8") as f:
                f.write('fn main() {\n    println("Hello from OmniLang!");\n}\n')

        print(f"Initialized OmniLang project in {os.path.abspath(target_dir)}")
        print(f"  Config: {config_path}")
        print(f"  Source: {src_dir}/")

        if errors:
            print("\nConfig warnings:")
            for err in errors:
                print(f"  - {err}")

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


# ════════════════════════════════════════════════════════════
# Format
# ════════════════════════════════════════════════════════════

def cmd_fmt(args):
    """Format an OmniLang source file."""
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    with open(args.file, "r", encoding="utf-8") as f:
        source = f.read()

    formatted = _format_omni(source, indent=args.indent)

    if args.check:
        if formatted != source:
            print(f"File {args.file} needs formatting.")
            sys.exit(1)
        else:
            print(f"File {args.file} is properly formatted.")
            return

    if formatted != source:
        with open(args.file, "w", encoding="utf-8") as f:
            f.write(formatted)
        print(f"Formatted {args.file}")
    else:
        print(f"File {args.file} is already formatted.")


def _format_omni(source: str, indent: int = 4) -> str:
    """Basic OmniLang code formatter.

    Performs:
    - Consistent indentation using spaces
    - Trailing whitespace removal
    - Single trailing newline
    - Brace-style normalization (opening brace on same line)
    """
    lines = source.split('\n')
    formatted_lines = []
    indent_level = 0
    indent_str = ' ' * indent

    for line in lines:
        stripped = line.rstrip()

        if not stripped:
            formatted_lines.append('')
            continue

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


# ════════════════════════════════════════════════════════════
# Serve
# ════════════════════════════════════════════════════════════

def cmd_serve(args):
    """Start the visual editor server."""
    # Lazy import — editor deps (fastapi, uvicorn) are optional
    from editor.server.main import cmd_serve as _serve
    _serve(args)


# ════════════════════════════════════════════════════════════
# Watch
# ════════════════════════════════════════════════════════════

def cmd_watch(args):
    """Watch .omni files and auto-rebuild on changes."""
    import difflib
    import time

    input_file = args.file
    target = args.target
    output_dir = args.output
    show_diff = args.diff
    all_targets = args.all

    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}", file=sys.stderr)
        sys.exit(1)

    targets_to_check = ALL_TARGETS if all_targets else [target]

    print(f"Watching {input_file} (target={target}, output={output_dir})")
    print("Press Ctrl+C to stop.")
    print("")

    last_mtime = os.path.getmtime(input_file)
    build_count = 0
    previous_outputs: dict[str, str] = {}

    if show_diff:
        print(f"  Diff mode: showing changes across {len(targets_to_check)} target(s)")
        print("")

    from omnilang.api import compile_omni

    try:
        while True:
            time.sleep(0.5)
            try:
                current_mtime = os.path.getmtime(input_file)
            except OSError:
                continue

            if current_mtime != last_mtime:
                last_mtime = current_mtime
                build_count += 1
                timestamp = time.strftime("%H:%M:%S")

                try:
                    with open(input_file, encoding="utf-8") as f:
                        source = f.read()
                except Exception:
                    continue

                results: dict[str, str] = {}
                errors: list[str] = []

                for tgt in targets_to_check:
                    try:
                        result = compile_omni(source, filepath=input_file, target=tgt,
                                              previous_files=previous_outputs.get(tgt))
                        if result.success and result.files:
                            combined = "\n".join(
                                f"// {fname}\n{content}"
                                for fname, content in sorted(result.files.items())
                            )
                            results[tgt] = combined
                            previous_outputs[tgt] = combined
                        elif result.errors:
                            errors.extend(e.message for e in result.errors)
                    except Exception as e:
                        errors.append(f"{tgt}: {e}")

                if errors:
                    print(f"[{timestamp}] #{build_count} FAILED: {'; '.join(errors[:3])}")
                    continue

                if show_diff:
                    print(f"[{timestamp}] #{build_count} Changed:")
                    for tgt in targets_to_check:
                        output = results.get(tgt, "")
                        prev = previous_outputs.get(f"_{tgt}_last_displayed", "")
                        if output != prev:
                            prev_lines = prev.split("\n") if prev else []
                            curr_lines = output.split("\n")
                            diff = list(difflib.unified_diff(
                                prev_lines, curr_lines,
                                fromfile=f"{tgt}/prev", tofile=f"{tgt}/current",
                                lineterm="",
                            ))
                            changed = len([l for l in diff if l.startswith("+") and not l.startswith("+++")])
                            removed = len([l for l in diff if l.startswith("-") and not l.startswith("---")])
                            if changed or removed:
                                print(f"  {tgt:12s} +{changed}/-{removed} lines changed")
                                for dline in diff[2:8]:
                                    if dline and not dline.startswith("@@"):
                                        print(f"    {dline[:100]}")
                            else:
                                print(f"  {tgt:12s} no change")
                            previous_outputs[f"_{tgt}_last_displayed"] = output
                else:
                    print(f"[{timestamp}] #{build_count} OK ({len(targets_to_check)} targets)")
    except KeyboardInterrupt:
        print(f"\nStopped. ({build_count} build(s))")




# ════════════════════════════════════════════════════════════
# Helpers
# ════════════════════════════════════════════════════════════

def _detect_ui_mode(source: str) -> bool:
    """Detect whether source contains OmniView UI components."""
    ui_keywords = ("component ", "store ", "theme ", "view {", "@if", "@for", "signal ", "computed ")
    has_ui = any(kw in source for kw in ui_keywords)
    has_ui = has_ui or bool(re.search(r'<[A-Z]', source))
    has_ui = has_ui or "import omniview" in source
    return has_ui


# ════════════════════════════════════════════════════════════
# Main
# ════════════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(
        prog="omnic",
        description="OmniLang compiler — transpile .omni to Rust, Dart, JS, Python, React, Vue, and more",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # ── build ──
    build_parser = subparsers.add_parser("build", help="Compile an .omni source file to target language(s)")
    build_parser.add_argument("file", help="Source file (.omni)")
    build_parser.add_argument(
        "--target", "-t",
        choices=ALL_TARGETS,
        help="Target language/framework (default: auto-detect react or python)",
    )
    build_parser.add_argument(
        "--output", "-o",
        default="./build",
        help="Output directory (default: ./build)",
    )

    # ── check ──
    check_parser = subparsers.add_parser(
        "check", help="Run static analysis (via Rust compiler)"
    )
    check_parser.add_argument("file", nargs="?", help="Source file (.omni) or project directory")
    check_parser.add_argument(
        "--format", "-f",
        choices=["terminal", "json", "github"],
        default="terminal",
        help="Output format (default: terminal)",
    )
    check_parser.add_argument(
        "--explain",
        metavar="CODE",
        help="Show detailed explanation for a diagnostic code (e.g., P0101)",
    )

    # ── init ──
    init_parser = subparsers.add_parser("init", help="Initialize a new OmniLang project")
    init_parser.add_argument("dir", nargs="?", default=".", help="Project directory (default: .)")
    init_parser.add_argument("--name", "-n", help="Project name (default: directory name)")

    # ── fix ──
    fix_parser = subparsers.add_parser("fix", help="Auto-fix diagnostics in source file")
    fix_parser.add_argument("file", help="Source file (.omni)")
    fix_parser.add_argument("--dry-run", action="store_true", help="Preview fixes without applying")
    fix_parser.add_argument("--rule", help="Only apply a specific rule")

    # ── run ──
    run_parser = subparsers.add_parser("run", help="Compile and run an OmniLang file")
    run_parser.add_argument("file", help="Source file (.omni)")
    run_parser.add_argument(
        "--target", "-t",
        choices=["rust", "dart", "js", "python"],
        default="python",
        help="Target language to run (default: python)",
    )

    # ── fmt ──
    fmt_parser = subparsers.add_parser("fmt", help="Format an OmniLang source file")
    fmt_parser.add_argument("file", help="Source file (.omni)")
    fmt_parser.add_argument("--check", action="store_true", help="Check formatting without modifying")
    fmt_parser.add_argument("--indent", type=int, default=4, help="Indentation size (default: 4)")

    # ── serve ──
    serve_parser = subparsers.add_parser("serve", help="Start the visual editor server")
    serve_parser.add_argument("--dir", required=True, help="Project directory to serve")
    serve_parser.add_argument("--port", type=int, default=5173, help="Server port (default: 5173)")
    serve_parser.add_argument("--vite-port", type=int, default=5174, help="Vite dev server port (default: 5174)")
    serve_parser.add_argument("--dev", action="store_true", help="Force Vite dev mode (even if production build exists)")
    serve_parser.add_argument("--no-frontend", action="store_true", help="Skip auto-starting Vite dev server")

    # ── watch ──
    watch_parser = subparsers.add_parser("watch", help="Watch .omni files and auto-rebuild on changes")
    watch_parser.add_argument("file", help="Source file (.omni)")
    watch_parser.add_argument(
        "--target", "-t",
        choices=ALL_TARGETS,
        default="react",
        help="Target language/framework (default: react)",
    )
    watch_parser.add_argument(
        "--output", "-o",
        default="./build",
        help="Output directory (default: ./build)",
    )
    watch_parser.add_argument("--diff", action="store_true", help="Show diff on changes")
    watch_parser.add_argument("--all", action="store_true", help="Build for all targets")

    args = parser.parse_args()

    if args.command == "build":
        cmd_build(args)
    elif args.command == "check":
        cmd_check(args)
    elif args.command == "fix":
        cmd_fix(args)
    elif args.command == "run":
        cmd_run(args)
    elif args.command == "init":
        cmd_init(args)
    elif args.command == "fmt":
        cmd_fmt(args)
    elif args.command == "serve":
        cmd_serve(args)
    elif args.command == "watch":
        cmd_watch(args)
    else:
        parser.print_help()

    sys.exit(0)


if __name__ == "__main__":
    main()
