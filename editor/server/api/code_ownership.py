"""
可逆代码所有权: @editor: managed / @editor: hand-written 区域解析。

当用户在画布上编辑视觉属性时，编辑器需要将修改回写到 .omni 源码。
为了避免覆盖用户手写的代码，源码使用标记注释来声明区域所有权：
  - @editor:managed    — 编辑器全权管理，画布修改直接覆盖
  - @editor:hand-written — 用户手写，画布修改不覆盖，弹出冲突面板
"""

import re
from dataclasses import dataclass

# Regex: matches comment-prefixed @editor: managed|hand-written markers
# Supports: //, #, /*, *, <!-- comment styles, with or without space after @editor:
_MARKER_RE = re.compile(
    r"^\s*(?://|#|/\*|\*|<!--)?\s*@editor:\s*(managed|hand-written)\s*(?:\*/|-->)?\s*$"
)


@dataclass
class OwnershipRegion:
    """源码中的一个所有权区域。"""
    type: str          # "managed" | "handwritten"
    start_line: int    # 0-based inclusive
    end_line: int      # 0-based inclusive
    content: str = ""  # 区域文本


def parse_ownership_regions(source: str) -> list[OwnershipRegion]:
    """解析源码中的所有 @editor:managed / @editor:hand-written 区域标记。

    未标记的区域默认为 managed。
    """
    lines = source.split("\n")
    regions: list[OwnershipRegion] = []
    current_type = "managed"  # 默认编辑器管理
    current_start = 0

    for i, line in enumerate(lines):
        m = _MARKER_RE.match(line)
        if m:
            marker_type = m.group(1).replace("-", "")  # normalize "hand-written" → "handwritten"
            if current_start < i:
                region_content = "\n".join(lines[current_start:i])
                regions.append(OwnershipRegion(
                    type=current_type,
                    start_line=current_start,
                    end_line=i - 1,
                    content=region_content,
                ))
            current_type = marker_type
            current_start = i + 1  # 标记行本身不属于区域内容

    # 最后一个区域
    if current_start < len(lines):
        region_content = "\n".join(lines[current_start:])
        regions.append(OwnershipRegion(
            type=current_type,
            start_line=current_start,
            end_line=len(lines) - 1,
            content=region_content,
        ))

    return regions


def apply_canvas_edit(
    source: str,
    edit_start_line: int,
    edit_end_line: int,
    new_content: str,
) -> str:
    """应用画布修改到源码，跳过 hand-written 区域。

    如果编辑区域与 hand-written 区域重叠，抛出 OwnershipConflictError。
    否则用 new_content 替换 [edit_start_line, edit_end_line] 区间。

    Args:
        source: 原始 .omni 源码
        edit_start_line: 编辑起始行 (0-based, inclusive)
        edit_end_line: 编辑结束行 (0-based, inclusive)
        new_content: 替换内容

    Returns:
        修改后的源码

    Raises:
        OwnershipConflictError: 编辑区域与手写区域重叠
    """
    regions = parse_ownership_regions(source)
    lines = source.split("\n")

    # 检查编辑区域是否与 hand-written 区域重叠
    for region in regions:
        if region.type != "handwritten":
            continue
        # 重叠检测
        if edit_start_line <= region.end_line and edit_end_line >= region.start_line:
            raise OwnershipConflictError(
                f"编辑区域 (行 {edit_start_line + 1}-{edit_end_line + 1}) "
                f"与手写区域 (行 {region.start_line + 1}-{region.end_line + 1}) 重叠"
            )

    # 无冲突 → 应用编辑
    new_lines = lines[:edit_start_line] + new_content.split("\n") + lines[edit_end_line + 1:]
    return "\n".join(new_lines)


def append_managed_region(source: str, content: str) -> str:
    """在源码末尾追加一个新的 managed 区域。"""
    return source.rstrip("\n") + "\n// @editor: managed\n" + content + "\n"


def replace_managed_region(
    source: str,
    region_index: int,
    new_content: str,
) -> str:
    """替换第 N 个 managed 区域的内容。

    Args:
        source: 原始源码
        region_index: managed 区域的索引（仅计数 managed 区域）
        new_content: 新内容

    Returns:
        修改后的源码

    Raises:
        ValueError: managed 区域索引不存在
    """
    regions = parse_ownership_regions(source)
    managed_indices = [
        i for i, r in enumerate(regions) if r.type == "managed"
    ]

    if region_index < 0 or region_index >= len(managed_indices):
        raise ValueError(
            f"Managed 区域索引 {region_index} 超出范围 (共 {len(managed_indices)} 个)"
        )

    target_idx = managed_indices[region_index]
    region = regions[target_idx]

    lines = source.split("\n")
    new_lines = (
        lines[:region.start_line]
        + new_content.split("\n")
        + lines[region.end_line + 1:]
    )
    return "\n".join(new_lines)


def get_handwritten_regions(source: str) -> list[OwnershipRegion]:
    """获取所有 hand-written 区域。"""
    return [r for r in parse_ownership_regions(source) if r.type == "handwritten"]


def get_managed_regions(source: str) -> list[OwnershipRegion]:
    """获取所有 managed 区域。"""
    return [r for r in parse_ownership_regions(source) if r.type == "managed"]


class OwnershipConflictError(Exception):
    """画布编辑与手写区域冲突。"""
    pass


# ── AST-anchored ownership (M6) ─────────────────────────


def parse_ownership_ast_nodes(
    source: str,
    filepath: str = "<input>",
) -> list[dict]:
    """解析所有权区域，锚定到 AST 节点 ID（M6 新增）。

    与 parse_ownership_regions() 不同，此函数尝试将所有权区域映射到
    解析后的 AST 节点（component/store/signal/action），使所有权在
    代码重构时保持稳定（不依赖行号）。

    Returns:
        list[dict]: 每个条目包含:
            - regionType: "managed" | "handwritten"
            - nodeId: AST 节点 ID
            - nodeType: "component" | "store" | "view" | "signal" | "action" | "fn" | "style"
            - fieldPath: JSON 指针路径
            - sourceLineStart: 0-based 起始行（回退显示用）
            - sourceLineEnd: 0-based 结束行（回退显示用）
    """
    from omnilang.api import parse_code_blocks

    # Parse line-based regions first
    regions = parse_ownership_regions(source)

    # Try to parse the AST to map line ranges to AST nodes
    code_blocks: list[dict] = []
    try:
        blocks = parse_code_blocks(source, filepath)
        code_blocks = [
            {
                "id": b.id, "blockType": b.blockType, "name": b.name,
                "parentStore": b.parentStore, "sourceLine": b.sourceLine,
            }
            for b in blocks
        ]
    except Exception:
        code_blocks = []

    # Build line → code block mapping
    # Each code block occupies approximately its declaration line
    block_by_line: dict[int, dict] = {}
    for cb in code_blocks:
        block_by_line[cb.get("sourceLine", 0)] = cb

    # Map each ownership region to the closest code block
    entries: list[dict] = []
    for r in regions:
        node_id = f"region-l{r.start_line}-l{r.end_line}"
        node_type = "view"
        field_path = f"source[{r.start_line}:{r.end_line}]"

        # Try to find an AST node that contains this line range
        for line_num, cb in sorted(block_by_line.items()):
            if line_num <= r.start_line <= line_num + 30:  # approximate node span
                node_id = cb.get("id", node_id)
                node_type = cb.get("blockType", "view")
                field_path = f"{cb.get('name', 'unknown')}.body"
                break

        entries.append({
            "regionType": r.type,
            "nodeId": node_id,
            "nodeType": node_type,
            "fieldPath": field_path,
            "sourceLineStart": r.start_line,
            "sourceLineEnd": r.end_line,
        })

    return entries
