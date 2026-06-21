# editor/server/annotation/editor_toml.py
"""Read and write .omni-editor.toml — project-level editor UI state."""

from __future__ import annotations
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class EditorTomlState:
    """Editor UI state stored in .omni-editor.toml."""
    active_file: str = ""
    active_mode: str = "design"         # "design" | "code"
    open_files: list[str] = field(default_factory=list)
    left_open: bool = True
    right_width: int = 320
    viewports: dict[str, dict[str, float]] = field(default_factory=dict)


def read_editor_toml(path: Path) -> EditorTomlState:
    """Read .omni-editor.toml. Returns defaults if file doesn't exist."""
    if not path.exists():
        return EditorTomlState()

    try:
        import tomllib  # Python 3.11+
    except ImportError:
        try:
            import tomli as tomllib  # Python < 3.11 fallback
        except ImportError:
            return EditorTomlState()

    with open(path, "rb") as f:
        data = tomllib.load(f)

    editor = data.get("editor", {})
    viewports_raw = editor.get("viewports", {})

    viewports: dict[str, dict[str, float]] = {}
    for filepath, vp in viewports_raw.items():
        viewports[filepath] = {
            "zoom": float(vp.get("zoom", 1.0)),
            "scroll_x": float(vp.get("scroll_x", 0)),
            "scroll_y": float(vp.get("scroll_y", 0)),
        }

    return EditorTomlState(
        active_file=editor.get("active_file", ""),
        active_mode=editor.get("active_mode", "design"),
        open_files=list(editor.get("open_files", {}).get("files", [])),
        left_open=editor.get("panels", {}).get("left_open", True),
        right_width=editor.get("panels", {}).get("right_width", 320),
        viewports=viewports,
    )


def write_editor_toml(path: Path, state: EditorTomlState) -> None:
    """Write .omni-editor.toml to disk."""
    lines: list[str] = []
    lines.append("# .omni-editor.toml — editor UI state, does not affect compilation")
    lines.append("")
    lines.append("[editor]")
    lines.append(f'active_file = "{state.active_file}"')
    lines.append(f'active_mode = "{state.active_mode}"')
    lines.append("")
    lines.append("[editor.open_files]")
    files_str = ", ".join(f'"{f}"' for f in state.open_files)
    lines.append(f"files = [{files_str}]")
    lines.append("")
    lines.append("[editor.panels]")
    lines.append(f"left_open = {str(state.left_open).lower()}")
    lines.append(f"right_width = {state.right_width}")
    lines.append("")

    for filepath, vp in state.viewports.items():
        lines.append(f'[editor.viewports."{filepath}"]')
        lines.append(f"zoom = {vp.get('zoom', 1.0)}")
        lines.append(f"scroll_x = {int(vp.get('scroll_x', 0))}")
        lines.append(f"scroll_y = {int(vp.get('scroll_y', 0))}")
        lines.append("")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")
