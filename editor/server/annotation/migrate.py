# editor/server/annotation/migrate.py
"""Migrate old .omni.json projects to new annotation-based format."""

from __future__ import annotations
import json
from pathlib import Path
from editor.server.annotation.editor_toml import write_editor_toml, EditorTomlState


def migrate_omni_json(json_path: Path, project_dir: Path) -> None:
    """Read old .omni.json, generate annotated .omni files, write .omni-editor.toml.

    Does NOT delete the original .omni.json — user can verify before deleting.
    """
    if not json_path.exists():
        raise FileNotFoundError(f".omni.json not found: {json_path}")

    with open(json_path, encoding="utf-8") as f:
        doc = json.load(f)

    design = doc.get("design", {})
    design_elements = design.get("elements", [])
    design_wires = design.get("wires", [])

    code = doc.get("code", {})
    code_elements = code.get("elements", [])
    code_wires = code.get("wires", [])

    config = doc.get("config", {})
    target = config.get("target", "react")
    tab_state = doc.get("tabState", {})

    # Generate src/App.omni with design elements
    src_dir = project_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    lines.append("// @code-tab type=logic")
    lines.append("import omniview;")
    lines.append("")

    component_name = doc.get("meta", {}).get("projectName", "App")
    lines.append(f"component {component_name} {{")
    lines.append("    view {")

    root_elements = [e for e in design_elements if e.get("parentId") is None]
    for el in root_elements:
        _emit_element(el, design_elements, lines, indent=2)

    lines.append("    }")
    lines.append("    style {")
    lines.append("    }")
    lines.append("}")

    main_omni = src_dir / f"{component_name}.omni"
    main_omni.write_text("\n".join(lines), encoding="utf-8")

    # Generate .omni-editor.toml
    state = EditorTomlState(
        active_file=f"src/{component_name}.omni",
        active_mode="design",
        open_files=[f"src/{component_name}.omni"],
    )
    toml_path = project_dir / ".omni-editor.toml"
    write_editor_toml(toml_path, state)

    # Generate stub .omnilang.toml if not exists
    omni_toml = project_dir / ".omnilang.toml"
    if not omni_toml.exists():
        omni_toml.write_text(f"""\
[project]
name = "{component_name.lower().replace(' ', '-')}"
version = "1.0.0"

[source]
dir = "src"

[output]
dir = "build"
""")

    print(f"Migration complete.")
    print(f"  Main file: {main_omni}")
    print(f"  Editor state: {toml_path}")
    print(f"  Original .omni.json preserved at: {json_path}")
    print(f"  Verify the migration, then delete .omni.json manually.")


def _emit_element(el: dict, all_elements: list[dict], lines: list[str], indent: int = 0) -> None:
    """Recursively emit a view element with @edit annotation."""
    pad = "    " * indent
    tag = el.get("tag", "div")
    eid = el.get("id", "")
    props = el.get("props", {})
    is_folded = el.get("isFolded", False)
    render_mode = el.get("renderMode", "dom")
    children_ids = el.get("children", [])

    edit_parts = ["// @edit"]
    if is_folded:
        edit_parts.append("folded")
    if render_mode == "canvas":
        edit_parts.append("render=canvas")
    lines.append(f"{pad}{' '.join(edit_parts)}")

    props_str = ""
    if eid:
        props_str += f' id="{eid}"'
    for k, v in props.items():
        if k == "class":
            props_str += f" class: {v}"
        elif k == "label":
            props_str += f' label="{v}"'
        elif not k.startswith("_"):
            if isinstance(v, str):
                props_str += f' {k}="{v}"'
            else:
                props_str += f" {k}={{{v}}}"
    props_str = props_str.strip()

    if children_ids:
        lines.append(f"{pad}<{tag} {props_str}>")
        for cid in children_ids:
            child = next((e for e in all_elements if e.get("id") == cid), None)
            if child:
                _emit_element(child, all_elements, lines, indent + 1)
        lines.append(f"{pad}</{tag}>")
    else:
        lines.append(f"{pad}<{tag} {props_str} />")
