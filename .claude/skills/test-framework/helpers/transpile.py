#!/usr/bin/env python3
"""Transpile .omni source to React via omniview API, scaffold Vite project.

Usage:
  python transpile.py --source <file.omni> --target react --output <dir>
  python transpile.py --source <file.omni> --target react --output <dir> --port 5201
"""

import argparse
import json
import os
import subprocess
import sys
import urllib.request
import urllib.error


API_BASE = os.environ.get("OMNI_API", "http://localhost:5173")


def compile_omni(source_path: str, target: str) -> dict:
    """POST /api/compile with .omni source, return response JSON."""
    with open(source_path, "r", encoding="utf-8") as f:
        source = f.read()

    body = json.dumps({"source": source, "target": target}).encode("utf-8")
    req = urllib.request.Request(
        f"{API_BASE}/api/compile",
        data=body,
        headers={"Content-Type": "application/json"},
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.URLError as e:
        print(f"ERROR: Cannot reach API at {API_BASE}/api/compile: {e}", file=sys.stderr)
        sys.exit(1)


def write_files(files: dict, output_dir: str):
    """Write transpiled files to output directory."""
    os.makedirs(output_dir, exist_ok=True)
    for fname, content in files.items():
        fpath = os.path.join(output_dir, fname)
        os.makedirs(os.path.dirname(fpath), exist_ok=True)
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  Wrote {fname} ({len(content)} chars)")


def scaffold_vite(output_dir: str, port: int, component_name: str):
    """Scaffold a minimal Vite + React project around the transpiled component."""
    # index.html
    index_html = f'''<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{component_name} — React</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/main.jsx"></script>
</body>
</html>'''

    with open(os.path.join(output_dir, "index.html"), "w", encoding="utf-8") as f:
        f.write(index_html)

    # main.jsx
    main_jsx = f'''import React from "react";
import ReactDOM from "react-dom/client";
import {{{component_name}}} from "./{component_name}";

ReactDOM.createRoot(document.getElementById("root")).render(
  React.createElement(React.StrictMode, null, React.createElement({component_name}))
);'''

    with open(os.path.join(output_dir, "main.jsx"), "w", encoding="utf-8") as f:
        f.write(main_jsx)

    # vite.config.js
    vite_config = f'''import {{ defineConfig }} from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({{
  plugins: [react()],
  server: {{ port: {port} }},
}});'''

    with open(os.path.join(output_dir, "vite.config.js"), "w", encoding="utf-8") as f:
        f.write(vite_config)

    # package.json
    package_json = {
        "name": "omnilang-react-test",
        "private": True,
        "version": "1.0.0",
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "vite build",
            "preview": "vite preview",
        },
        "dependencies": {
            "react": "^18.2.0",
            "react-dom": "^18.2.0",
            "zustand": "^4.5.0",
        },
        "devDependencies": {
            "@vitejs/plugin-react": "^4.2.0",
            "vite": "^5.4.0",
        },
    }

    with open(os.path.join(output_dir, "package.json"), "w", encoding="utf-8") as f:
        json.dump(package_json, f, indent=2)

    print(f"  Scaffolded Vite project (port={port})")


def install_deps(output_dir: str):
    """Run npm install in output directory."""
    node_modules = os.path.join(output_dir, "node_modules")
    if os.path.exists(node_modules):
        print("  node_modules exists, skipping npm install")
        return

    print("  Installing npm dependencies...")
    result = subprocess.run(
        ["npm", "install"],
        cwd=output_dir,
        capture_output=True,
        text=True,
        shell=(sys.platform == "win32"),
    )
    if result.returncode != 0:
        print(f"  WARNING: npm install failed:\n{result.stderr}")
    else:
        print("  npm install complete")


def main():
    parser = argparse.ArgumentParser(description="Transpile .omni → React + Vite scaffold")
    parser.add_argument("--source", required=True, help="Path to .omni source file")
    parser.add_argument("--target", default="react", help="Target language (default: react)")
    parser.add_argument("--output", required=True, help="Output directory for React project")
    parser.add_argument("--port", type=int, default=5201, help="Vite dev server port (default: 5201)")
    args = parser.parse_args()

    print(f"Transpiling {args.source} → {args.target}")
    print(f"  API: {API_BASE}")

    # Step 1: Compile
    result = compile_omni(args.source, args.target)
    if not result.get("success"):
        errors = result.get("errors", [])
        print(f"ERROR: Compile failed with {len(errors)} error(s):")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)

    files = result.get("files", {})
    print(f"  Compile OK — {len(files)} file(s): {', '.join(files.keys())}")

    # Step 2: Write transpiled files
    write_files(files, args.output)

    # Step 3: Determine component name from the first .jsx/.tsx file
    component_name = "App"
    for fname in files:
        if fname.endswith((".jsx", ".tsx")):
            base = os.path.splitext(os.path.basename(fname))[0]
            if base not in ("index", "main"):
                component_name = base
                break

    # Step 4: Scaffold Vite project
    scaffold_vite(args.output, args.port, component_name)

    # Step 5: Install deps
    install_deps(args.output)

    print(f"\nDone! React app ready at {args.output}")
    print(f"  cd {args.output} && npx vite --port {args.port}")


if __name__ == "__main__":
    main()
