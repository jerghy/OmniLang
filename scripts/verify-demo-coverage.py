#!/usr/bin/env python3
"""覆盖审计 v3：语义集成员 × 实现状态 × demo 使用。
只统计「支持/部分」成员的展示缺口（未支持 = 白名单外硬诊断，无需展示）。
"""
import re
from pathlib import Path

REPO = Path(r"D:\space\OmniLang")
UNIVERSE = REPO / "docs" / "superpowers" / "specs" / "semantic-universe.md"
DEMO = REPO / "test" / "projects" / "learn-omniview" / "src"

text = UNIVERSE.read_text(encoding="utf-8")
demo_text = "\n".join(p.read_text(encoding="utf-8") for p in DEMO.rglob("*.omni"))
parts = re.split(r"^## 域 (\d+):", text, flags=re.M)

def camel(name: str) -> str:
    return "".join(p[:1].upper() + p[1:] for p in re.split(r"[-_]", name) if p)

def usage(domain: str, name: str) -> int:
    if domain == "1":  # 可动画属性：只查 @motion 块内的 from/to 属性
        return len(re.findall(rf"\b{re.escape(name)}\s*:", demo_text))
    if domain == "2":
        return len(re.findall(rf"\.{re.escape(name)}\s*\(", demo_text))
    if domain == "3":
        # React 拼写为内单词大写（MouseEnter/DragStart/TransitionEnd）——IGNORECASE 覆盖两种拼写
        return len(re.findall(rf"@on{re.escape(name)}\s*=", demo_text, re.IGNORECASE)) + \
               len(re.findall(rf"(?<!@)on{re.escape(name)}\s*=", demo_text, re.IGNORECASE))
    if domain == "4":
        if name == "value":
            return len(re.findall(rf"\.value\b", demo_text))
        return len(re.findall(rf"\.{re.escape(name)}\s*\(", demo_text))
    if domain == "5":  # token 声明类型：`token name: TYPE =`
        return len(re.findall(rf"\btoken\s+[a-zA-Z]+\s*:\s*{re.escape(name)}\b", demo_text))
    if domain == "6":
        return len(re.findall(rf"{re.escape(name)}\b", demo_text))
    if domain == "7":
        if name == "drag":
            return len(re.findall(r"@drag\s*=", demo_text))
        if name == "dragEnd":
            return len(re.findall(r"@dragEnd\s*=", demo_text))
        if name == "ref":
            return len(re.findall(r"@ref\s*=|(?<![\w@])ref:\s*", demo_text))
        if name in ("focus", "scroll-x", "scroll-y"):
            return len(re.findall(rf"{re.escape(name)}\s*=\s*{{", demo_text))
        if name == "style 动态绑定":
            return len(re.findall(r"style:\s*\{", demo_text))
        if name == "布尔属性":
            return len(re.findall(
                r"\b(?:controls|disabled|readonly|required|checked|selected|autofocus|multiple|loop|muted|async|defer|hidden|autocomplete|autoplay)\s*=",
                demo_text,
            ))
    if domain == "10":  # 反馈模态：语义集条目名带 @ 前缀（指令），showToast 为函数调用形态
        if name == "showToast":
            return len(re.findall(r"showToast\s*\(", demo_text))
        if name in ("@modal", "焦点陷阱"):  # 焦点陷阱是 @modal 附随行为——随 @modal 使用展示
            return len(re.findall(r"@modal\s*=", demo_text))
        if name == "@skeleton":
            return len(re.findall(r"@skeleton\b", demo_text))
        if name == "@errorBoundary":
            return len(re.findall(r"@errorBoundary\b", demo_text))
    return 0

print("=" * 78)
print("learn-omniview 覆盖审计 v3（支持/部分成员 × demo 使用）")
print("=" * 78)
all_gaps = []
for i in range(1, len(parts), 2):
    num = parts[i].strip()
    rows = []
    for line in parts[i + 1].splitlines():
        m = re.match(r"^\|\s*([^|]+?)\s*\|[^|]*\|([^|]*)\|", line)
        if m:
            n = m.group(1).strip().strip("`")
            st = m.group(2).strip()
            if n and n not in ("语义集条目", "语义", "---") and set(n) - set("-: "):
                rows.append((n, st))
    # 状态列可能带括注（如 支持（v0：无手动关闭））——按「支持/部分」前缀归一
    sup = [(n, s) for n, s in rows if s.split("（")[0].strip() in ("支持", "部分")]
    if not sup:
        print(f"\n── 域 {num}: （无支持/部分成员——表格未解析？共 {len(rows)} 行）")
        continue
    print(f"\n── 域 {num}: 支持/部分 {len(sup)} 成员")
    gaps, shown = [], []
    for n, s in sup:
        c = usage(num, n)
        (gaps if c == 0 else shown).append((n, s, c))
    if gaps:
        print(f"  ❌ 未展示 ({len(gaps)}): " + ", ".join(f"{n}[{s}]" for n, s, _ in gaps))
    if shown:
        print(f"  ✅ 已展示 ({len(shown)}): " + ", ".join(f"{n}×{c}" for n, _, c in shown))
    all_gaps.extend((num, n, s) for n, s, _ in gaps)

# 无合法元素展示形态的部分支持成员（window 级/宿主动画）——审计豁免
EXEMPT = {
    ("3", "resize"),  # window 级事件——元素绑定不合法（React 无元素 onResize）；DSL 无 window 绑定表面
    ("3", "animationstart"), ("3", "animationend"), ("3", "animationiteration"),
    # 动画族——触发需宿主 CSS @keyframes（DSL 无）；绑定可写但展示为「需宿主动画」说明面板
    ("4", "capturePointer"), ("4", "releasePointer"),
    # ADR-073 决策 3：显式 capturePointer 降级为编译器内部细节，demo 源码不再书写——
    # 指针捕获意图由 ElementRefDemo 的 @drag/@dragEnd 绑定承担（域 7 已登记，意图形态
    # 由 @drag 承担同一演示语义）
}
all_gaps = [g for g in all_gaps if (g[0], g[1]) not in EXEMPT]

print("\n" + "=" * 78)
print(f"合计缺口（支持/部分未展示）: {len(all_gaps)}（豁免 {len(EXEMPT)} 项无合法元素形态）")
for num, n, s in all_gaps:
    print(f"  域{num}: {n} [{s}]")
