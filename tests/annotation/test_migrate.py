# tests/annotation/test_migrate.py
"""Tests for the omni.json -> annotation format migration."""

import json
import tempfile
from pathlib import Path
from editor.server.annotation.migrate import migrate_omni_json

OLD_OMNI_JSON = {
    "schemaVersion": "2.0.0",
    "meta": {"projectName": "test-project"},
    "sourceCode": "",
    "design": {
        "elements": [
            {
                "id": "e1", "nodeType": "view", "tag": "Button",
                "props": {"label": "Click", "class": "primary"},
                "children": [], "parentId": None,
                "isFolded": False, "renderMode": "dom",
            },
        ],
        "wires": [],
    },
    "code": {
        "elements": [
            {
                "id": "cb1", "nodeType": "codeblock", "tag": "signal",
                "props": {"name": "count", "type": "i32"},
                "children": [], "parentId": None,
                "nodeCategory": "variable",
            },
        ],
        "wires": [],
    },
    "components": [],
    "canvasLayout": {"positions": {}},
    "tabState": {"openTabs": ["main"], "activeTab": "main"},
    "config": {"target": "react", "autoSave": False},
}

class TestMigrate:
    def test_migration_creates_omni_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)
            json_path = project_dir / "project.omni.json"
            json_path.write_text(json.dumps(OLD_OMNI_JSON))

            migrate_omni_json(json_path, project_dir)

            main_file = project_dir / "src" / "test-project.omni"
            assert main_file.exists()
            content = main_file.read_text()
            assert "// @omni-file type=logic" in content

    def test_migration_generates_editor_toml(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)
            json_path = project_dir / "project.omni.json"
            json_path.write_text(json.dumps(OLD_OMNI_JSON))

            migrate_omni_json(json_path, project_dir)

            toml_path = project_dir / ".omni-editor.toml"
            assert toml_path.exists()

    def test_migration_uses_project_name_for_filename(self):
        """File name comes from meta.projectName, not hardcoded 'App'."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)
            json_path = project_dir / "project.omni.json"
            doc = dict(OLD_OMNI_JSON)
            doc["meta"] = {"projectName": "MyApp"}
            json_path.write_text(json.dumps(doc))

            migrate_omni_json(json_path, project_dir)

            main_file = project_dir / "src" / "MyApp.omni"
            assert main_file.exists()
