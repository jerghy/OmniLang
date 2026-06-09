"""OmniView CLI — omnic command-line tool.

Commands:
    omnic build <file.omni> [--target react]  Compile .omni to target framework
    omnic check <file.omni>                  Static analysis / lint
    omnic fmt <file.omni>                     Format .omni source (placeholder)
"""

import sys
import os
import json
from pathlib import Path


def find_grammar_path() -> str:
    """Find the grammar file relative to this module."""
    return str(Path(__file__).parent / "grammar.lark")


def _find_line_numbers(source: str) -> dict[str, int]:
    """Find line numbers of component and store definitions in source."""
    line_map = {}
    for i, line in enumerate(source.split('\n'), 1):
        stripped = line.strip()
        for prefix in ('component ', 'store '):
            if stripped.startswith(prefix):
                name = stripped.split()[1].split('{')[0].strip()
                line_map[name] = i
                break
    return line_map


def build_command(args: list[str]) -> int:
    """Compile .omni files to target framework code."""
    if not args:
        print("Error: No input file specified.", file=sys.stderr)
        print("Usage: omnic build <file.omni> [--target react]", file=sys.stderr)
        return 1

    input_file = args[0]
    target = "react"  # Phase 0 default
    output_dir = "build_output"
    generate_sourcemap = False

    # Parse flags
    i = 1
    while i < len(args):
        if args[i] == "--target" and i + 1 < len(args):
            target = args[i + 1]
            i += 2
        elif args[i] == "--output" and i + 1 < len(args):
            output_dir = args[i + 1]
            i += 2
        elif args[i] == "--sourcemap":
            generate_sourcemap = True
            i += 1
        else:
            i += 1

    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}", file=sys.stderr)
        return 1

    try:
        # Read source
        with open(input_file, "r", encoding="utf-8") as f:
            source = f.read()

        # Detect mode: has import omniview or UI keywords → UI mode, else pure logic
        has_ui_keywords = any(kw in source for kw in
            ("component ", "store ", "theme ", "view {", "@if", "@for", "<", "signal ", "computed "))
        has_import_omniview = "import omniview" in source

        if not has_import_omniview and not has_ui_keywords:
            # Pure logic mode: use OmniLang parser + codegen
            from omnilang.lexer import create_parser
            from omnilang.parser import OmniTransformer
            parser = create_parser()
            tree = parser.parse(source)
            program = OmniTransformer().transform(tree)
            print(f"Pure logic mode (target={target}). OmniLang constructs detected.")
            print(f"Pure logic codegen is under development. AST parsed successfully.")
            return 0

        # UI mode: use OmniView parser
        from omniview.preprocessor import preprocess
        pp_result = preprocess(source)

        from omnilang.lexer import create_omniview_parser
        from omniview.parser import OmniViewTransformer

        parser = create_omniview_parser()
        tree = parser.parse(pp_result.processed_source)
        transformer = OmniViewTransformer(block_map=pp_result.blocks)
        program = transformer.transform(tree)

        # Codegen routing: has_omniview → UI codegen
        # Backward compat: no import + components present → implicit omniview (warning)
        if not program.has_omniview:
            if program.components or program.stores:
                print(f"  Warning: Add 'import omniview;' at the top of the file. Assuming implicit omniview for now.", file=sys.stderr)

        # Semantic analysis
        from omniview.semantic.analyzer import SemanticAnalyzer
        analyzer = SemanticAnalyzer()
        result = analyzer.analyze(program)

        if result.has_errors:
            print(f"Semantic analysis found {result.error_count} error(s):", file=sys.stderr)
            for d in result.diagnostics:
                if d.level == "ERROR":
                    print(f"  {d}", file=sys.stderr)
            if result.error_count > 0:
                return 1

        # Lower to IR
        from omniview.lowering import IRFixture
        lower = IRFixture()
        ir_map = lower.lower_program(program)

        # Generate code
        if target == "react":
            from omniview.codegen.react import ReactCodegen
            codegen = ReactCodegen()
        elif target == "vue":
            from omniview.codegen.vue import VueCodegen
            codegen = VueCodegen()
        elif target == "dioxus":
            from omniview.codegen.dioxus import DioxusCodegen
            codegen = DioxusCodegen()
        elif target == "flutter":
            from omniview.codegen.flutter import FlutterCodegen
            codegen = FlutterCodegen()
        else:
            print(f"Error: Unknown target '{target}'. Supported: react, vue, dioxus, flutter", file=sys.stderr)
            return 1

        # Find source line numbers for accurate /* .omni:N */ comments
        source_lines = _find_line_numbers(source)

        os.makedirs(output_dir, exist_ok=True)

        for name, ir in ir_map.items():
            line_no = source_lines.get(name, 1)
            if "states" in ir or "view" in ir:
                result = codegen.generate_component(ir, ir_map, source_line=line_no)
            else:
                result = codegen.generate_store(ir, source_line=line_no)

            for filename, content in result.files.items():
                out_path = Path(output_dir) / filename
                os.makedirs(out_path.parent, exist_ok=True)
                with open(out_path, "w", encoding="utf-8") as f:
                    f.write(content)
                print(f"  Generated: {out_path}")

                # Generate source map if requested
                if generate_sourcemap:
                    from omniview.sourcemap import SourceMapper
                    mapper = SourceMapper(
                        omni_filename=os.path.basename(input_file),
                        generated_filename=filename,
                    )
                    # Map each line of generated code back to .omni source
                    import re
                    omni_comment_re = re.compile(r"\.omni:(\d+)")
                    for gen_line_no, gen_line in enumerate(content.split("\n")):
                        # Look for .omni:N comments in various formats:
                        #   // .omni:7 -- component Foo
                        #   /* .omni:12 */
                        omni_match = omni_comment_re.search(gen_line)
                        if omni_match:
                            omni_line = int(omni_match.group(1))
                            mapper.add_mapping(
                                generated_line=gen_line_no,
                                generated_col=0,
                                original_line=omni_line - 1,  # 0-based
                                original_col=0,
                            )
                    map_path = str(out_path) + ".map"
                    mapper.write(map_path)
                    # Add source map comment to the generated file
                    with open(out_path, "a", encoding="utf-8") as f:
                        f.write(f"\n//# sourceMappingURL={filename}.map\n")

            for warning in result.warnings:
                print(f"  Warning: {warning}", file=sys.stderr)
            for error in result.errors:
                print(f"  Error: {error}", file=sys.stderr)

        print(f"Build complete. Output: {output_dir}/")

    except Exception as e:
        msg = str(e)
        friendly = _format_parse_error(msg) if msg else "Unknown error"
        print(f"Build failed: {friendly}", file=sys.stderr)
        return 1

    return 0


def check_command(args: list[str]) -> int:
    """Run static analysis on .omni files."""
    if not args:
        print("Error: No input file specified.", file=sys.stderr)
        print("Usage: omnic check <file.omni>", file=sys.stderr)
        return 1

    input_file = args[0]
    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}", file=sys.stderr)
        return 1

    try:
        with open(input_file, "r", encoding="utf-8") as f:
            source = f.read()

        from lark import Lark
        from omniview.parser import OmniViewTransformer
        from omniview.semantic.analyzer import SemanticAnalyzer

        grammar_path = find_grammar_path()
        with open(grammar_path, "r", encoding="utf-8") as f:
            grammar = f.read()

        parser = create_omniview_parser()
        tree = parser.parse(source)
        transformer = OmniViewTransformer()
        program = transformer.transform(tree)

        analyzer = SemanticAnalyzer()
        result = analyzer.analyze(program)

        # Report
        print(f"File: {input_file}")
        print(f"Components: {len(program.components)}")
        print(f"Stores: {len(program.stores)}")
        print(f"Themes: {len(program.themes)}")
        print(f"Symbols: {len(result.symbols)}")
        print(f"Dependencies: {sum(len(v) for v in result.dependencies.values())}")
        print(f"Diagnostics: {len(result.diagnostics)}")

        if result.diagnostics:
            errors = [d for d in result.diagnostics if d.level == "ERROR"]
            warnings = [d for d in result.diagnostics if d.level == "WARNING"]
            infos = [d for d in result.diagnostics if d.level == "INFO"]
            print(f"  Errors: {len(errors)}")
            for d in errors:
                print(f"    {d}")
            print(f"  Warnings: {len(warnings)}")
            for d in warnings:
                print(f"    {d}")
            print(f"  Info: {len(infos)}")

        return result.error_count

    except Exception as e:
        msg = str(e)
        friendly = _format_parse_error(msg) if msg else "Unknown error"
        print(f"Check failed: {friendly}", file=sys.stderr)
        return 1


def _format_parse_error(error_msg: str) -> str:
    """Translate Lark parser errors into user-friendly messages."""
    import re
    # "No terminal matches 'X' in the current parser context"
    m = re.search(r"No terminal matches '([^']+)'", error_msg)
    if m:
        char = m.group(1)
        if char == '.':
            return f"Syntax error: unexpected '{char}'. Did you mean to use spread syntax '...'? Spread is supported inside array literals: [...array, item]"
        return f"Syntax error: unexpected character '{char}'"
    # "Unexpected token"
    if "Unexpected token" in error_msg:
        return f"Syntax error: {error_msg.split(chr(10))[0]}"
    # "Expected one of"
    m = re.search(r"at line (\d+)", error_msg)
    if m:
        line = m.group(1)
        return f"Syntax error at line {line}: {error_msg.split(chr(10))[0]}"
    return f"Parse error: {error_msg.split(chr(10))[0]}"


def init_command(args: list[str]) -> int:
    """Scaffold a new OmniView React project."""
    target = "react"
    project_name = "omniview-app"

    i = 0
    while i < len(args):
        if args[i] == "--target" and i + 1 < len(args):
            target = args[i + 1]
            i += 2
        elif not args[i].startswith("--"):
            project_name = args[i]
            i += 1
        else:
            i += 1

    project_dir = Path(project_name)
    if project_dir.exists():
        print(f"Error: Directory '{project_name}' already exists.", file=sys.stderr)
        return 1

    os.makedirs(project_dir / "src", exist_ok=True)

    # package.json
    package_json = {
        "name": project_name,
        "private": True,
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "vite build",
            "preview": "vite preview"
        },
        "dependencies": {
            "react": "^18.3.1",
            "react-dom": "^18.3.1",
            "zustand": "^4.5.0"
        },
        "devDependencies": {
            "@types/react": "^18.3.0",
            "@types/react-dom": "^18.3.0",
            "@vitejs/plugin-react": "^4.3.0",
            "typescript": "^5.5.0",
            "vite": "^5.4.0"
        }
    }
    with open(project_dir / "package.json", "w", encoding="utf-8") as f:
        json.dump(package_json, f, indent=2)
        f.write("\n")

    # tsconfig.json
    tsconfig = {
        "compilerOptions": {
            "target": "ES2020",
            "module": "ESNext",
            "moduleResolution": "bundler",
            "jsx": "react-jsx",
            "strict": True,
            "esModuleInterop": True,
            "skipLibCheck": True,
            "noEmit": True
        },
        "include": ["src/**/*.ts", "src/**/*.tsx"],
        "exclude": ["node_modules"]
    }
    with open(project_dir / "tsconfig.json", "w", encoding="utf-8") as f:
        json.dump(tsconfig, f, indent=2)
        f.write("\n")

    # vite.config.ts — with OmniView plugin support
    vite_plugin_ref = ""
    if target == "react":
        vite_plugin_ref = 'import react from "@vitejs/plugin-react";\n'
    elif target == "vue":
        vite_plugin_ref = 'import vue from "@vitejs/plugin-vue";\n'

    with open(project_dir / "vite.config.ts", "w", encoding="utf-8") as f:
        f.write('import { defineConfig } from "vite";\n')
        f.write(vite_plugin_ref)
        f.write('// OmniView Vite plugin (optional — enables .omni HMR):\n')
        f.write('// import omniView from "./plugins/vite-plugin-omniview/index.js";\n')
        f.write('\n')
        f.write('export default defineConfig({\n')
        if target == "react":
            f.write('  plugins: [react()],\n')
        elif target == "vue":
            f.write('  plugins: [vue()],\n')
        f.write('  // Add omniView({ target: "' + target + '" }) for .omni HMR:\n')
        f.write('  // plugins: [react(), omniView({ target: "react" })],\n')
        f.write('});\n')

    # Copy Vite plugin to project if it doesn't exist
    plugin_src = Path(__file__).parent / "plugins" / "vite"
    plugin_dst = project_dir / "plugins" / "vite-plugin-omniview"
    if plugin_src.exists() and not plugin_dst.exists():
        os.makedirs(plugin_dst, exist_ok=True)
        import shutil
        for fname in ["index.js", "index.d.ts", "package.json"]:
            src_file = plugin_src / fname
            if src_file.exists():
                shutil.copy(src_file, plugin_dst / fname)

    # index.html
    with open(project_dir / "index.html", "w", encoding="utf-8") as f:
        f.write('<!DOCTYPE html>\n')
        f.write('<html lang="en">\n')
        f.write('<head>\n')
        f.write('  <meta charset="UTF-8" />\n')
        f.write('  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\n')
        f.write(f'  <title>{project_name}</title>\n')
        f.write('</head>\n')
        f.write('<body>\n')
        f.write('  <div id="root"></div>\n')
        f.write('  <script type="module" src="/src/main.tsx"></script>\n')
        f.write('</body>\n')
        f.write('</html>\n')

    # src/vite-env.d.ts
    with open(project_dir / "src" / "vite-env.d.ts", "w", encoding="utf-8") as f:
        f.write('/// <reference types="vite/client" />\n')

    # src/main.tsx
    with open(project_dir / "src" / "main.tsx", "w", encoding="utf-8") as f:
        f.write('import React from "react";\n')
        f.write('import ReactDOM from "react-dom/client";\n')
        f.write('// Import your OmniView-generated component here:\n')
        f.write('// import { App } from "./App";\n')
        f.write('\n')
        f.write('ReactDOM.createRoot(document.getElementById("root")!).render(\n')
        f.write('  <React.StrictMode>\n')
        f.write('    {/* <App /> */}\n')
        f.write('  </React.StrictMode>\n')
        f.write(');\n')

    # .gitignore
    with open(project_dir / ".gitignore", "w", encoding="utf-8") as f:
        f.write('node_modules\n')
        f.write('dist\n')
        f.write('.DS_Store\n')

    print(f"OmniView React project created: {project_dir}/")
    print(f"")
    print(f"  cd {project_name}")
    print(f"  npm install")
    print(f"  omnic build your-component.omni --output src/")
    print(f"  npm run dev")
    return 0


def watch_command(args: list[str]) -> int:
    """Watch .omni files and auto-rebuild on changes."""
    import time

    if not args:
        print("Error: No input file specified.", file=sys.stderr)
        print("Usage: omnic watch <file.omni> [--target react|vue] [--output dir]", file=sys.stderr)
        return 1

    input_file = args[0]
    target = "react"
    output_dir = "build_output"

    i = 1
    while i < len(args):
        if args[i] == "--target" and i + 1 < len(args):
            target = args[i + 1]
            i += 2
        elif args[i] == "--output" and i + 1 < len(args):
            output_dir = args[i + 1]
            i += 2
        else:
            i += 1

    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}", file=sys.stderr)
        return 1

    print(f"Watching {input_file} (target={target}, output={output_dir})")
    print("Press Ctrl+C to stop.")
    print("")

    last_mtime = os.path.getmtime(input_file)
    build_count = 0

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
                print(f"[{timestamp}] #{build_count} Rebuilding...", end=" ", flush=True)

                try:
                    ret = build_command([input_file, "--target", target, "--output", output_dir])
                    if ret == 0:
                        print("OK")
                    else:
                        print(f"FAILED (exit code {ret})")
                except Exception as e:
                    print(f"ERROR: {e}")
    except KeyboardInterrupt:
        print(f"\nStopped. ({build_count} build(s))")

    return 0


def fmt_command(args: list[str]) -> int:
    """Format .omni source files (Phase 1)."""
    print("omnic fmt: Not yet implemented (Phase 1 feature)")
    return 0


def main():
    if len(sys.argv) < 2:
        print("OmniView CLI — omnic v0.3.0", file=sys.stderr)
        print("", file=sys.stderr)
        print("Commands:", file=sys.stderr)
        print("  omnic init   [name]          Scaffold a new React project", file=sys.stderr)
        print("  omnic build  <file.omni>     Compile to target framework", file=sys.stderr)
        print("  omnic check  <file.omni>     Static analysis", file=sys.stderr)
        print("  omnic watch  <file.omni>     Watch and auto-rebuild", file=sys.stderr)
        print("  omnic fmt    <file.omni>     Format source (Phase 1)", file=sys.stderr)
        print("  omnic gen-extern <file.d.ts> Generate Rust extern from .d.ts", file=sys.stderr)
        print("", file=sys.stderr)
        print("Options:", file=sys.stderr)
        print("  --target <framework>        Target framework (default: react)", file=sys.stderr)
        print("  --output <dir>              Output directory (default: build_output)", file=sys.stderr)
        return 1

    command = sys.argv[1]
    args = sys.argv[2:]

    if command == "init":
        return init_command(args)
    elif command == "build":
        return build_command(args)
    elif command == "check":
        return check_command(args)
    elif command == "watch":
        return watch_command(args)
    elif command == "fmt":
        return fmt_command(args)
    elif command == "gen-extern":
        from omniview.gen_extern import gen_extern_command
        return gen_extern_command(args)
    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        print("Available: init, build, check, watch, fmt", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
