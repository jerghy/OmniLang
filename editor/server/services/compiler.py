"""Compiler service helpers — extracted from editor/server/main.py."""


def serialize_tree(nodes: list) -> list:
    """Recursively serialize ViewTreeNode to a JSON-safe list of dicts."""
    result = []
    for n in nodes:
        result.append({
            "id": n.id,
            "tag": n.tag,
            "props": n.props,
            "testId": n.test_id,
            "sourceLine": n.source_line,
            "children": serialize_tree(n.children) if n.children else [],
        })
    return result
