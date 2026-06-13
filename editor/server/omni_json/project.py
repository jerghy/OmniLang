"""OmniLang editor — project file management (.omni.json splitting/merging).

Per design doc §2.1, each component gets its own .omni + .omni.json file.
The project.omni.json manifest references all components/stores and stores
the shared editor state (elements, wires, search index, canvas layout).
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from editor.server.omni_json.schema import OmniJsonDocument, OmniMetadata
from editor.server.omni_json.serializer import OmniJsonSerializer


class OmniProjectManager:
    """Manages a multi-file OmniLang editor project.

    Directory layout:
        project.omni.json          — manifest (shared state + component references)
        src/
          ComponentName.omni       — per-component human-readable source
          ComponentName.omni.json  — per-component editor snapshot
          StoreName.omni           — per-store source
          StoreName.omni.json      — per-store editor snapshot
    """

    MANIFEST_FILENAME = "project.omni.json"
    COMPONENT_DIR = "src"

    def __init__(self, project_dir: str | Path):
        self.project_dir = Path(project_dir)
        self.serializer = OmniJsonSerializer()

    # ── Manifest ──────────────────────────────────────────

    @property
    def manifest_path(self) -> Path:
        return self.project_dir / self.MANIFEST_FILENAME

    def manifest_exists(self) -> bool:
        return self.manifest_path.exists()

    def load_manifest(self) -> OmniJsonDocument | None:
        """Load project manifest if it exists."""
        if not self.manifest_exists():
            return None
        try:
            return self.serializer.load(self.manifest_path)
        except Exception:
            return None

    def save_manifest(self, doc: OmniJsonDocument) -> None:
        """Save the project manifest."""
        self.serializer.save(doc, self.manifest_path)

    # ── Split ─────────────────────────────────────────────

    def split_document(
        self,
        doc: OmniJsonDocument,
        exporter=None,  # OmniJsonExporter (optional, for .omni generation)
    ) -> dict[str, Path]:
        """Split a monolithic document into per-component files.

        Returns:
            dict: { "ComponentName": Path to .omni.json, ... }
        """
        written: dict[str, Path] = {}
        src_dir = self.project_dir / self.COMPONENT_DIR
        src_dir.mkdir(parents=True, exist_ok=True)

        # Write each component to its own .omni.json
        for comp in doc.components:
            comp_name = comp.name
            comp_path = src_dir / f"{comp_name}.omni.json"
            # Build a mini-document for this component
            comp_doc = OmniJsonDocument(
                schema_version=doc.schema_version,
                meta=OmniMetadata(
                    project_name=comp_name,
                    omni_version=doc.meta.omni_version,
                    created_at=doc.meta.created_at,
                    updated_at=datetime.now(timezone.utc).isoformat(),
                ),
                components=[comp],
                stores=[],
                themes=[],
                imports=doc.imports,
                elements=[e for e in doc.elements if e.id.startswith(f"comp-{comp_name}") or comp_name in e.tag],
                wires=[],
                search_index=doc.search_index,
            )
            self.serializer.save(comp_doc, comp_path)
            written[comp_name] = comp_path

        # Write each store to its own .omni.json
        for store in doc.stores:
            store_name = store.name
            store_path = src_dir / f"{store_name}.omni.json"
            store_doc = OmniJsonDocument(
                schema_version=doc.schema_version,
                meta=OmniMetadata(
                    project_name=store_name,
                    omni_version=doc.meta.omni_version,
                    created_at=doc.meta.created_at,
                    updated_at=datetime.now(timezone.utc).isoformat(),
                ),
                components=[],
                stores=[store],
                themes=[],
                imports=doc.imports,
                elements=[e for e in doc.elements if e.store_name == store_name],
                wires=[],
                search_index=doc.search_index,
            )
            self.serializer.save(store_doc, store_path)
            written[store_name] = store_path

        # Write the manifest (references + shared state)
        # Strip components/stores from manifest (they're in separate files now)
        manifest = OmniJsonDocument(
            schema_version=doc.schema_version,
            meta=doc.meta,
            components=[],  # Referenced via filesystem
            stores=[],      # Referenced via filesystem
            themes=doc.themes,
            imports=doc.imports,
            elements=doc.elements,
            wires=doc.wires,
            ownership_regions=doc.ownership_regions,
            search_index=doc.search_index,
            canvas_layout=doc.canvas_layout,
        )
        self.save_manifest(manifest)

        return written

    # ── Merge ─────────────────────────────────────────────

    def merge_documents(self) -> OmniJsonDocument | None:
        """Reconstruct monolithic document from split files.

        Reads the manifest and all per-component/per-store files.
        """
        manifest = self.load_manifest()
        if not manifest:
            return None

        src_dir = self.project_dir / self.COMPONENT_DIR
        if not src_dir.exists():
            return manifest  # No component files yet

        # Scan for per-component files
        all_components = list(manifest.components)
        all_stores = list(manifest.stores)

        for comp_file in sorted(src_dir.glob("*.omni.json")):
            # Skip store files (handled below)
            try:
                sub_doc = self.serializer.load(comp_file)
            except Exception:
                continue

            all_components.extend(sub_doc.components)
            all_stores.extend(sub_doc.stores)

        return OmniJsonDocument(
            schema_version=manifest.schema_version,
            meta=manifest.meta,
            components=all_components,
            stores=all_stores,
            themes=manifest.themes,
            imports=manifest.imports,
            elements=manifest.elements,
            wires=manifest.wires,
            ownership_regions=manifest.ownership_regions,
            search_index=manifest.search_index,
            canvas_layout=manifest.canvas_layout,
        )

    # ── File listing ──────────────────────────────────────

    def list_project_files(self) -> list[dict[str, Any]]:
        """List all project files (for FileExplorer)."""
        files: list[dict[str, Any]] = []

        # Manifest
        if self.manifest_exists():
            files.append({
                "name": self.MANIFEST_FILENAME,
                "path": self.MANIFEST_FILENAME,
                "type": "file",
                "ext": ".json",
                "meta": "Project Manifest",
            })

        # Component/Store files
        src_dir = self.project_dir / self.COMPONENT_DIR
        if src_dir.exists():
            for f in sorted(src_dir.iterdir()):
                if f.name.startswith("."):
                    continue
                ext = f.suffix
                kind = "Editor Data" if ext == ".json" else "Source"
                files.append({
                    "name": f.name,
                    "path": f"{self.COMPONENT_DIR}/{f.name}".replace("\\", "/"),
                    "type": "file",
                    "ext": ext,
                    "meta": kind,
                })

        return files
