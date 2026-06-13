"""Wire connection validators — pure functions for port-type compatibility,
cycle detection, and duplicate prevention in the visual programming canvas.

These operate on plain dicts (matching .omni.json ElementEntry / WireEntry shapes)
so they work server-side and can be ported to TypeScript for client-side use.
"""

from __future__ import annotations

from typing import Any


# ═══════════════════════════════════════════════════════════════
# Type compatibility
# ═══════════════════════════════════════════════════════════════

# Known type compatibility: from_type can connect to to_type if
# to_type is in the compatible set or either is "any".
_TYPE_COMPAT: dict[str, set[str]] = {
    "i32": {"i32", "i64", "f64", "bool", "String", "any"},
    "i64": {"i32", "i64", "f64", "bool", "String", "any"},
    "f64": {"i32", "i64", "f64", "bool", "String", "any"},
    "bool": {"i32", "i64", "f64", "bool", "String", "any"},
    "String": {"i32", "i64", "f64", "bool", "String", "any"},
    "any": {"i32", "i64", "f64", "bool", "String", "any"},
}


def validate_port_types(
    from_type: str,
    to_type: str,
    strict: bool = False,
) -> tuple[bool, str | None]:
    """Check if a wire from `from_type` to `to_type` is type-compatible.

    Args:
        from_type: Source port type (e.g. "i32", "String")
        to_type: Target port type
        strict: If True, requires exact type match (for critical paths)

    Returns:
        (is_valid, error_message_or_None)
    """
    if strict:
        if from_type != to_type:
            return False, f"Strict type mismatch: {from_type} → {to_type}"
        return True, None

    compatible = _TYPE_COMPAT.get(from_type, set())
    if to_type not in compatible and "any" not in (from_type, to_type):
        return False, f"Type mismatch: cannot connect {from_type} → {to_type}"
    return True, None


# ═══════════════════════════════════════════════════════════════
# Direction validation
# ═══════════════════════════════════════════════════════════════


def validate_port_directions(
    from_ports: list[dict[str, str]],
    to_ports: list[dict[str, str]],
    from_port_name: str,
    to_port_name: str,
) -> tuple[bool, str | None]:
    """Check that the source port is an output and the target port is an input.

    Args:
        from_ports: Source element's port definitions [{name, direction, type}]
        to_ports: Target element's port definitions
        from_port_name: Name of the source port being connected
        to_port_name: Name of the target port being connected

    Returns:
        (is_valid, error_message_or_None)
    """
    from_port = next((p for p in from_ports if p.get("name") == from_port_name), None)
    to_port = next((p for p in to_ports if p.get("name") == to_port_name), None)

    if from_port is None:
        return False, f"Source port '{from_port_name}' not found on element"
    if to_port is None:
        return False, f"Target port '{to_port_name}' not found on element"

    if from_port.get("direction") != "out":
        return False, f"Source port '{from_port_name}' must be an output (direction=out)"
    if to_port.get("direction") != "in":
        return False, f"Target port '{to_port_name}' must be an input (direction=in)"

    return True, None


# ═══════════════════════════════════════════════════════════════
# Duplicate detection
# ═══════════════════════════════════════════════════════════════


def has_duplicate_wire(
    wires: list[dict[str, Any]],
    from_id: str,
    from_port: str,
    to_id: str,
    to_port: str,
    exclude_id: str | None = None,
) -> bool:
    """Check if a wire with the same endpoints already exists.

    Args:
        wires: Existing wire list
        from_id: Source element ID
        from_port: Source port name
        to_id: Target element ID
        to_port: Target port name
        exclude_id: Wire ID to exclude from check (for update operations)

    Returns:
        True if a duplicate wire already exists
    """
    for w in wires:
        wid = w.get("id", "")
        if exclude_id and wid == exclude_id:
            continue
        if (w.get("fromId") == from_id and w.get("fromPort") == from_port and
                w.get("toId") == to_id and w.get("toPort") == to_port):
            return True
    return False


def has_self_loop(from_id: str, to_id: str) -> bool:
    """Check if a wire connects an element to itself."""
    return from_id == to_id


# ═══════════════════════════════════════════════════════════════
# Cycle detection (directed graph)
# ═══════════════════════════════════════════════════════════════


def detect_cycle(
    wires: list[dict[str, Any]],
    new_from_id: str,
    new_to_id: str,
) -> bool:
    """Check if adding a wire from new_from_id → new_to_id would create a cycle.

    Uses DFS from new_to_id back to new_from_id through existing wire connections.

    Args:
        wires: Existing wire list
        new_from_id: Proposed source element ID
        new_to_id: Proposed target element ID

    Returns:
        True if the new wire would create a cycle
    """
    # Build adjacency list: from_id → set of to_ids
    adj: dict[str, set[str]] = {}
    for w in wires:
        fid = w.get("fromId", "")
        tid = w.get("toId", "")
        if fid not in adj:
            adj[fid] = set()
        adj[fid].add(tid)

    # Add the proposed wire
    if new_from_id not in adj:
        adj[new_from_id] = set()
    adj[new_from_id].add(new_to_id)

    # DFS from new_to_id — if we can reach new_from_id, there's a cycle
    visited: set[str] = set()

    def dfs(node: str) -> bool:
        if node == new_from_id:
            return True
        if node in visited:
            return False
        visited.add(node)
        for neighbor in adj.get(node, set()):
            if dfs(neighbor):
                return True
        return False

    return dfs(new_to_id)


# ═══════════════════════════════════════════════════════════════
# Combined validation
# ═══════════════════════════════════════════════════════════════


class WireValidationError(Exception):
    """Raised when a wire connection is invalid."""


def validate_wire_connection(
    elements: list[dict[str, Any]],
    wires: list[dict[str, Any]],
    from_id: str,
    from_port: str,
    to_id: str,
    to_port: str,
    exclude_wire_id: str | None = None,
    strict_types: bool = False,
) -> list[str]:
    """Run all wire validation checks. Returns list of error messages (empty = valid).

    This is the main entry point — call this before adding a wire to the canvas.
    """
    errors: list[str] = []

    # Self-loop check
    if has_self_loop(from_id, to_id):
        errors.append(f"Cannot connect element '{from_id}' to itself")

    # Duplicate check
    if has_duplicate_wire(wires, from_id, from_port, to_id, to_port, exclude_wire_id):
        errors.append(
            f"Duplicate wire: {from_id}:{from_port} → {to_id}:{to_port} already exists"
        )

    # Find source and target elements
    from_el = next((e for e in elements if e.get("id") == from_id), None)
    to_el = next((e for e in elements if e.get("id") == to_id), None)

    if from_el is None:
        errors.append(f"Source element '{from_id}' not found")
        return errors  # Can't continue without source
    if to_el is None:
        errors.append(f"Target element '{to_id}' not found")
        return errors  # Can't continue without target

    # Direction check
    from_ports = from_el.get("ports", [])
    to_ports = to_el.get("ports", [])
    dir_ok, dir_err = validate_port_directions(from_ports, to_ports, from_port, to_port)
    if not dir_ok:
        errors.append(dir_err)

    # Type compatibility check (if ports have type info)
    from_port_def = next((p for p in from_ports if p.get("name") == from_port), None)
    to_port_def = next((p for p in to_ports if p.get("name") == to_port), None)
    if from_port_def and to_port_def:
        from_type = from_port_def.get("type", "any")
        to_type = to_port_def.get("type", "any")
        type_ok, type_err = validate_port_types(from_type, to_type, strict=strict_types)
        if not type_ok:
            errors.append(type_err)

    # Cycle detection
    if detect_cycle(wires, from_id, to_id):
        errors.append(
            f"Cycle detected: adding {from_id} → {to_id} would create a circular dependency"
        )

    return errors
