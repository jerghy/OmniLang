"""CLI entry point for the OmniLang compiler."""

import argparse
import sys
import os

from omnilang.parser import parse
from omnilang.semantic import analyze
from omnilang.codegen import generate


TARGET_EXTENSIONS = {
    "rust": ".rs",
    "dart": ".dart",
    "js": ".js",
    "python": ".py",
}


def cmd_build(args):
    """Compile .omni source to target language(s)."""
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    try:
        # Read source
        with open(args.file, "r", encoding="utf-8") as f:
            source = f.read()

        # Compile pipeline
        prog = parse(source, filename=args.file)

        source_lines = {args.file: source.split("\n")}
        type_map, rename_map = analyze(prog, source_lines)

        # Generate code
        base_name = os.path.splitext(os.path.basename(args.file))[0]
        targets = [args.target] if args.target else ["rust", "dart", "js", "python"]

        os.makedirs(args.output, exist_ok=True)

        for target in targets:
            code = generate(prog, target, rename_map, type_map)
            ext = TARGET_EXTENSIONS[target]
            out_path = os.path.join(args.output, f"{base_name}{ext}")
            with open(out_path, "w", encoding="utf-8") as f:
                f.write(code)
            print(f"  -> {out_path}")

        print(f"Compiled successfully to {len(targets)} target(s).")

    except SystemExit:
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


def cmd_check(args):
    """Run static analysis on .omni source."""
    if args.explain:
        cmd_explain(args.explain)
        return

    if not args.file or not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    try:
        from omnilang.analyze.diagnostic import (
            Diagnostic, DiagnosticSummary,
        )

        # Read source
        with open(args.file, "r", encoding="utf-8") as f:
            source = f.read()

        # Parse
        prog = parse(source, filename=args.file)
        source_lines = {args.file: source.split("\n")}

        # Semantic analysis (existing v0.1 pipeline — catches type errors)
        try:
            type_map, rename_map = analyze(prog, source_lines)
        except SystemExit:
            # analyze() calls sys.exit(1) on errors — catch and report
            print("Semantic analysis found errors.", file=sys.stderr)
            sys.exit(1)

        # ── Analyze pipeline ──
        from omnilang.analyze.registry import build_default_registry, AnalyzeContext
        from omnilang.analyze.renderer import (
            render_terminal_plain, render_json, render_github,
        )
        from omnilang.config import load_config

        # Build registry and load config
        registry = build_default_registry()
        config = load_config()

        # Create analysis context
        ctx = AnalyzeContext(
            program=prog,
            source_lines=source_lines,
            targets=config.targets,
            type_map=type_map,
            rename_map=rename_map,
        )

        # Run all enabled rules
        diagnostics: list[Diagnostic] = []
        for rule in registry.list_enabled(config):
            diagnostics.extend(rule.check(ctx))

        # Apply suppression directives
        from omnilang.analyze.suppress import apply_suppression
        rule_overrides = {}
        if config and hasattr(config, 'analyze'):
            rule_overrides = dict(getattr(config.analyze, 'rules', {}))
        diagnostics = apply_suppression(diagnostics, source_lines, rule_overrides)

        # Summarize and render
        summary = DiagnosticSummary.from_diagnostics(diagnostics)

        if args.format == "json":
            render_json(diagnostics)
        elif args.format == "github":
            render_github(diagnostics)
        else:
            render_terminal_plain(diagnostics, source_lines)

        if summary.total == 0:
            print("No issues found.")
            sys.exit(0)
        else:
            print(summary)
            sys.exit(summary.exit_code())

    except SystemExit:
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


def cmd_explain(code: str):
    """Show detailed explanation for a diagnostic code."""
    import os
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


def cmd_fix(args):
    """Auto-fix diagnostics in a source file."""
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    with open(args.file, "r", encoding="utf-8") as f:
        source = f.read()

    prog = parse(source, filename=args.file)
    source_lines = {args.file: source.split("\n")}
    type_map, rename_map = analyze(prog, source_lines)

    from omnilang.analyze.registry import build_default_registry, AnalyzeContext
    from omnilang.analyze.fix_engine import build_default_fix_engine

    registry = build_default_registry()
    ctx = AnalyzeContext(program=prog, source_lines=source_lines, type_map=type_map, rename_map=rename_map)
    diagnostics = []
    for rule in registry.list_all():
        if args.rule and rule.code != args.rule:
            continue
        diagnostics.extend(rule.check(ctx))

    if not diagnostics:
        print("No issues to fix.")
        return

    engine = build_default_fix_engine()
    if args.dry_run:
        print(f"Would fix {len(diagnostics)} issue(s):")
        for d in diagnostics:
            print(f"  {d.code}: {d.message}")
    else:
        modified = engine.apply_all(diagnostics, source)
        with open(args.file, "w", encoding="utf-8") as f:
            f.write(modified)
        print(f"Fixed {len(diagnostics)} issue(s) in {args.file}")


def cmd_run(args):
    """Compile and immediately run an OmniLang file."""
    import subprocess
    import tempfile
    import os
    if not os.path.exists(args.file):
        print(f"Error: file not found: {args.file}", file=sys.stderr)
        sys.exit(1)
    with open(args.file, "r", encoding="utf-8") as f:
        source = f.read()
    prog = parse(source, filename=args.file)
    source_lines = {args.file: source.split("\n")}
    type_map, rename_map = analyze(prog, source_lines)
    code = generate(prog, args.target, rename_map, type_map)

    tmpdir = tempfile.mkdtemp(prefix="omni_run_")
    base = os.path.splitext(os.path.basename(args.file))[0]
    ext = TARGET_EXTENSIONS[args.target]
    out = os.path.join(tmpdir, f"{base}{ext}")
    with open(out, "w", encoding="utf-8") as f:
        f.write(code)

    if args.target == "python":
        _result = subprocess.run(["python", out], capture_output=False)
    elif args.target == "js":
        _result = subprocess.run(["node", out], capture_output=False)
    elif args.target == "dart":
        _result = subprocess.run(["dart", "run", out], capture_output=False)
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


def cmd_init(args):
    """Initialize a new OmniLang project."""
    import os

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

        # Skip empty lines
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

        # NOTE: ; terminates statements but doesn't affect indent

    # Ensure single trailing newline
    result = '\n'.join(formatted_lines).rstrip() + '\n'
    return result


# ── Terminal rendering ────────────────────────────────

def _render_terminal(diagnostics, source_lines):
    """Render diagnostics with colored terminal output."""
    for diag in diagnostics:
        label = f"{diag.severity.name}: {diag.code}"

        print(f"{label}")
        print(f"  --> {diag.location}")
        print("   |")
        if diag.location.file in source_lines:
            file_lines = source_lines[diag.location.file]
            line_no = diag.location.line
            if line_no <= len(file_lines):
                src_line = file_lines[line_no - 1].rstrip()
                print(f"{line_no:>3} | {src_line}")
                col = diag.location.column
                span_end = diag.location.span[3]
                indicator = " " * (col - 1) + "^" * max(span_end - col, 1)
                print(f"   | {indicator}")
        print(f"   | {diag.message}")
        if diag.help:
            print(f"   = help: {diag.help}")
        for suggestion in diag.suggestions:
            print(f"   = suggestion: {suggestion.message}")
        print()


def _render_json(diagnostics, summary):
    """Render diagnostics as JSON."""
    import json

    output = {
        "diagnostics": [
            {
                "code": d.code,
                "severity": d.severity.value,
                "category": d.category,
                "message": d.message,
                "location": {
                    "file": d.location.file,
                    "line": d.location.line,
                    "column": d.location.column,
                },
                "help": d.help,
                "suggestions": [
                    {"message": s.message, "applicable": s.applicable}
                    for s in d.suggestions
                ],
            }
            for d in diagnostics
        ],
        "summary": {
            "errors": summary.errors,
            "warnings": summary.warnings,
            "ports": summary.ports,
            "styles": summary.styles,
            "perfs": summary.perfs,
            "secs": summary.secs,
            "hints": summary.hints,
            "total": summary.total,
        },
    }
    print(json.dumps(output, indent=2))


def _render_github(diagnostics):
    """Render diagnostics as GitHub Actions workflow commands.

    Format: ::error file=main.omni,line=5,col=10::E0101: type mismatch
    """
    severity_map = {
        "error": "error",
        "warning": "warning",
        "port": "warning",
        "style": "notice",
        "perf": "warning",
        "sec": "error",
        "hint": "notice",
    }
    for diag in diagnostics:
        gh_sev = severity_map.get(diag.severity.value, "warning")
        print(
            f"::{gh_sev} "
            f"file={diag.location.file},"
            f"line={diag.location.line},"
            f"col={diag.location.column}"
            f"::{diag.code}: {diag.message}"
        )


def main():
    parser = argparse.ArgumentParser(
        prog="omnic",
        description="OmniLang compiler — transpile .omni to Rust, Dart, JS, or Python",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # ── build ─────────────────────────────────────────
    build_parser = subparsers.add_parser("build", help="Compile an OmniLang source file")
    build_parser.add_argument("file", help="Source file (.omni)")
    build_parser.add_argument(
        "--target", "-t",
        choices=["rust", "dart", "js", "python"],
        help="Target language (omit for all four)",
    )
    build_parser.add_argument(
        "--output", "-o",
        default="./build",
        help="Output directory (default: ./build)",
    )

    # ── check ─────────────────────────────────────────
    check_parser = subparsers.add_parser(
        "check", help="Run static analysis (type + portability + lint)"
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

    # ── init ──────────────────────────────────────────
    init_parser = subparsers.add_parser("init", help="Initialize a new OmniLang project")
    init_parser.add_argument("dir", nargs="?", default=".", help="Project directory (default: .)")
    init_parser.add_argument("--name", "-n", help="Project name (default: directory name)")

    # ── fix ──────────────────────────────────────────
    fix_parser = subparsers.add_parser("fix", help="Auto-fix diagnostics in source file")
    fix_parser.add_argument("file", help="Source file (.omni)")
    fix_parser.add_argument("--dry-run", action="store_true", help="Preview fixes without applying")
    fix_parser.add_argument("--rule", help="Only apply a specific rule")

    # ── run ──────────────────────────────────────────
    run_parser = subparsers.add_parser("run", help="Compile and run an OmniLang file")
    run_parser.add_argument("file", help="Source file (.omni)")
    run_parser.add_argument("--target", "-t", choices=["rust","dart","js","python"], default="python",
                           help="Target language to run (default: python)")

    # ── fmt ──────────────────────────────────────────
    fmt_parser = subparsers.add_parser("fmt", help="Format an OmniLang source file")
    fmt_parser.add_argument("file", help="Source file (.omni)")
    fmt_parser.add_argument("--check", action="store_true", help="Check formatting without modifying")
    fmt_parser.add_argument("--indent", type=int, default=4, help="Indentation size (default: 4)")

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
    else:
        parser.print_help()

    sys.exit(0)


if __name__ == "__main__":
    main()
