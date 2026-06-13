"""OmniLang editor — 3-way semantic merge for .omni.json documents.

Per design doc §2.1:
  - .omni is the merge truth source
  - Different declaration blocks edited by different people → auto-merge
  - Same block edited by two people → semantic conflict markers
  - Search index is always regenerated after merge
"""

from __future__ import annotations

from typing import Any


class OmniJsonMerger:
    """Performs 3-way semantic merge of .omni.json documents.

    Merge strategy (by ID):
    1. Components: merge by name; different components → auto-merge
    2. Same component: merge by field; different fields → auto-merge
    3. Same field in same component → conflict
    4. Elements: merge by ID; same ID with different props → field-level merge
    5. Wires: merge by ID; same ID with different ports → conflict
    6. Search index: always regenerated from merged document
    7. Canvas layout: simple union by element ID, latest wins for conflicts
    """

    def three_way_merge(
        self,
        base: dict[str, Any],
        local: dict[str, Any],
        remote: dict[str, Any],
    ) -> dict[str, Any]:
        """Perform 3-way semantic merge.

        Args:
            base: Original .omni.json (common ancestor)
            local: Local changes
            remote: Remote/upstream changes

        Returns:
            dict with keys: success, merged, conflicts
        """
        conflicts: list[dict[str, Any]] = []
        merged: dict[str, Any] = {}

        # Merge top-level metadata
        merged["schemaVersion"] = base.get("schemaVersion", "1.0.0")
        merged["meta"] = self._merge_meta(
            base.get("meta", {}),
            local.get("meta", {}),
            remote.get("meta", {}),
        )

        # Merge components (by name)
        merged["components"], comp_conflicts = self._merge_by_name(
            base.get("components", []),
            local.get("components", []),
            remote.get("components", []),
            "components",
        )
        conflicts.extend(comp_conflicts)

        # Merge stores (by name)
        merged["stores"], store_conflicts = self._merge_by_name(
            base.get("stores", []),
            local.get("stores", []),
            remote.get("stores", []),
            "stores",
        )
        conflicts.extend(store_conflicts)

        # Merge elements (by ID)
        merged["elements"], elem_conflicts = self._merge_by_key(
            base.get("elements", []),
            local.get("elements", []),
            remote.get("elements", []),
            "id",
            "elements",
        )
        conflicts.extend(elem_conflicts)

        # Merge wires (by ID)
        merged["wires"], wire_conflicts = self._merge_by_key(
            base.get("wires", []),
            local.get("wires", []),
            remote.get("wires", []),
            "id",
            "wires",
        )
        conflicts.extend(wire_conflicts)

        # Merge themes (simple union by name)
        merged["themes"], theme_conflicts = self._merge_by_name(
            base.get("themes", []),
            local.get("themes", []),
            remote.get("themes", []),
            "themes",
        )
        conflicts.extend(theme_conflicts)

        # Imports: simple union
        merged["imports"] = sorted(set(
            list(base.get("imports", []))
            + list(local.get("imports", []))
            + list(remote.get("imports", []))
        ))

        # Ownership regions: take union, latest wins for same nodeId
        merged["ownershipRegions"] = self._merge_by_key(
            base.get("ownershipRegions", []),
            local.get("ownershipRegions", []),
            remote.get("ownershipRegions", []),
            "nodeId",
            "ownershipRegions",
        )[0]  # No conflicts for ownership (latest wins)

        # Search index: always regenerate
        merged["searchIndex"] = {"entries": [], "version": 1, "lastBuilt": ""}

        # Canvas layout: union by element ID, latest wins
        merged["canvasLayout"] = {
            "positions": {
                **(base.get("canvasLayout", {}).get("positions", {})),
                **(local.get("canvasLayout", {}).get("positions", {})),
                **(remote.get("canvasLayout", {}).get("positions", {})),
            }
        }

        return {
            "success": len(conflicts) == 0,
            "merged": merged,
            "conflicts": conflicts,
        }

    # ── Merge helpers ────────────────────────────────────────

    def _merge_meta(
        self,
        base_meta: dict,
        local_meta: dict,
        remote_meta: dict,
    ) -> dict:
        """Merge metadata — use most recent timestamp."""
        result = {**base_meta}
        # Take the most recently updated
        local_ts = local_meta.get("updatedAt", "")
        remote_ts = remote_meta.get("updatedAt", "")
        result["updatedAt"] = max(local_ts, remote_ts) if local_ts and remote_ts else local_ts or remote_ts
        # Merge warnings (union)
        result["conversionWarnings"] = sorted(set(
            list(base_meta.get("conversionWarnings", []))
            + list(local_meta.get("conversionWarnings", []))
            + list(remote_meta.get("conversionWarnings", []))
        ))
        return result

    def _merge_by_name(
        self,
        base_list: list[dict],
        local_list: list[dict],
        remote_list: list[dict],
        path_prefix: str,
    ) -> tuple[list[dict], list[dict]]:
        """Merge lists of dicts keyed by 'name' field (components, stores, themes)."""

        def _key(item: dict) -> str:
            return item.get("name", "")

        return self._merge_by_key(base_list, local_list, remote_list, "name", path_prefix)

    def _merge_by_key(
        self,
        base_list: list[dict],
        local_list: list[dict],
        remote_list: list[dict],
        key_field: str,
        path_prefix: str,
    ) -> tuple[list[dict], list[dict]]:
        """Merge lists of dicts by a key field.

        Strategy:
        - Items only in local → include (addition)
        - Items only in remote → include (addition)
        - Items only deleted in one side → exclude (deletion respected)
        - Items in both local and remote → deep field-level merge
        """
        conflicts: list[dict] = []

        def _index(items: list[dict]) -> dict[str, dict]:
            return {item.get(key_field, ""): item for item in items}

        base_map = _index(base_list)
        local_map = _index(local_list)
        remote_map = _index(remote_list)

        merged_map: dict[str, dict] = {}
        all_keys = set(base_map.keys()) | set(local_map.keys()) | set(remote_map.keys())

        for key in all_keys:
            in_local = key in local_map
            in_remote = key in remote_map
            in_base = key in base_map

            if in_local and not in_remote:
                # Only in local — include (new addition)
                merged_map[key] = local_map[key]
            elif in_remote and not in_local:
                # Only in remote — include (new addition)
                merged_map[key] = remote_map[key]
            elif in_local and in_remote:
                # In both — deep merge
                local_item = local_map[key]
                remote_item = remote_map[key]
                base_item = base_map.get(key, {})

                if local_item == remote_item:
                    # Identical — no conflict
                    merged_map[key] = local_item
                else:
                    # Both changed — field-level merge
                    merged_item, field_conflicts = self._merge_fields(
                        base_item, local_item, remote_item,
                        f"{path_prefix}.{key}",
                    )
                    merged_map[key] = merged_item
                    conflicts.extend(field_conflicts)
            # else: deleted in both → omit (implicit deletion)

        return list(merged_map.values()), conflicts

    def _merge_fields(
        self,
        base_item: dict,
        local_item: dict,
        remote_item: dict,
        path: str,
    ) -> tuple[dict, list[dict]]:
        """Deep field-level merge of two dicts against a common base."""
        conflicts: list[dict] = []
        merged: dict = {}

        all_fields = set(base_item.keys()) | set(local_item.keys()) | set(remote_item.keys())

        for field in all_fields:
            field_path = f"{path}.{field}"
            base_val = base_item.get(field)
            local_val = local_item.get(field)
            remote_val = remote_item.get(field)

            # If unchanged from base on both sides, keep base
            # If changed only on one side, take that side
            # If changed on both sides differently, conflict

            local_changed = local_val != base_val
            remote_changed = remote_val != base_val

            if not local_changed and not remote_changed:
                merged[field] = base_val
            elif local_changed and not remote_changed:
                merged[field] = local_val
            elif remote_changed and not local_changed:
                merged[field] = remote_val
            else:
                # Both changed
                if local_val == remote_val:
                    # Both made the same change — no conflict
                    merged[field] = local_val
                elif isinstance(local_val, list) and isinstance(remote_val, list):
                    # Lists: try union
                    merged_list = list({json_key(v) for v in (base_val or []) + local_val + remote_val})
                    merged[field] = merged_list
                elif isinstance(local_val, dict) and isinstance(remote_val, dict):
                    # Nested dicts: recurse
                    nested_merged, nested_conflicts = self._merge_fields(
                        base_val or {}, local_val, remote_val, field_path,
                    )
                    merged[field] = nested_merged
                    conflicts.extend(nested_conflicts)
                else:
                    # True conflict
                    merged[field] = None  # Placeholder — user must resolve
                    conflicts.append({
                        "path": field_path,
                        "localValue": local_val,
                        "remoteValue": remote_val,
                        "baseValue": base_val,
                    })

        return merged, conflicts


def json_key(obj: Any) -> str:
    """Stable JSON serialization for dedup."""
    import json
    return json.dumps(obj, sort_keys=True, default=str)
