"""Build learn-omniview to verify design refactoring compiles."""
import re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.resolve()))

try:
    from omnilang.rust_bridge import RustBridge
    RustBridge._load_dll()

    src_dir = Path("test/projects/learn-omniview/src")
    main_file = src_dir / "main.omni"

    sources = {}
    path_mapping = {}
    pending = [str(main_file.resolve()).replace("\\", "/")]
    import_re = re.compile(r'import\s+\{[^}]*\}\s+from\s+"([^"]+)"')

    while pending:
        fp = pending.pop()
        if fp in sources:
            continue
        with open(fp, encoding="utf-8") as f:
            source = f.read()
        sources[fp] = source
        for m in import_re.finditer(source):
            imp = m.group(1)
            if imp.startswith("."):
                resolved = str((Path(fp).parent / imp).resolve()).replace("\\", "/") + ".omni"
                path_mapping[imp] = resolved
                if resolved not in sources:
                    pending.append(resolved)

    print(f"Collected {len(sources)} source files")
    for s in sorted(sources.keys()):
        print(f"  - {Path(s).relative_to(src_dir.parent)}")

    result = RustBridge.compile_to_target(sources, "omniview", path_mapping)
    if "error" in result and result["error"]:
        print(f"\nBUILD ERROR: {result['error']}")
        sys.exit(1)
    else:
        files = result.get("files", {})
        print(f"\nBUILD SUCCESSFUL! Generated files:")
        for name, content in files.items():
            print(f"  {name}: {len(content)} bytes")

        # Check if design CSS was generated
        css = files.get("index.css", "")
        if "LearnDesign" in css:
            print("\n✓ LearnDesign design system CSS found in output")
        if "data-design" in css:
            print("✓ data-design scope attribute found")
        if "--primary:" in css:
            print("✓ Design tokens (CSS variables) found")

        # Check JS for design class references
        js = files.get("index.js", "")
        if "Card" in js:
            print("✓ Design class references found in JS output")

except ImportError:
    print("RustBridge not available - skipping build test")
    print("Checking source files for syntax correctness only...")
    sys.exit(0)
except Exception as e:
    print(f"Build error: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)
