#!/usr/bin/env python3
"""覆盖审计 v3：语义集成员 × 实现状态 × demo 使用。
只统计「支持/部分」成员的展示缺口（未支持 = 白名单外硬诊断，无需展示）。
"""
import re
import sys
from pathlib import Path

# Windows GBK 控制台兜底：✅/❌ 等非 ASCII 输出不再触发 UnicodeEncodeError
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

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
    if domain == "8":  # 表单：@form 指令 + validate 键 + 表单状态
        if name == "@form":
            return len(re.findall(r"@form\b", demo_text))
        if name in ("required", "format", "min", "max", "pattern", "custom"):
            # validate: { required: true, ... } 键形式 + 裸属性形式（布尔属性，无 = 值）
            return len(re.findall(rf"\b{re.escape(name)}\s*:", demo_text)) + len(
                re.findall(rf"(?m)^\s*{re.escape(name)}\s*$", demo_text)
            )
        if name in ("formErrors", "formValid", "formSubmitting"):
            # DSL 表层拼写为 form.errors / form.valid / form.submitting
            # （非内单词大写的 formErrors）——按表层形态匹配（同域 10 约定）
            surface = {"formErrors": "errors", "formValid": "valid", "formSubmitting": "submitting"}[name]
            return len(re.findall(rf"form\.{re.escape(surface)}\b", demo_text))
    if domain == "9":  # 数据加载：@load 指令 + loading/error/data 三态 + 重试
        if name == "@load":
            return len(re.findall(r"@load\s*=", demo_text))
        if name == "data":
            return len(re.findall(r"data\b(?!-)", demo_text))  # (?!-) 排除 data-testid/data-type 等
        if name in ("loading", "error", "retry"):
            return len(re.findall(rf"\b{re.escape(name)}\b", demo_text))
        # 批次 4（2026-08-11）域 9 深化成员（semantic-universe.md 域 9 登记先行）
        if name == "@load 命名实例（多例）":  # DataFetching direct/api 双例
            return len(re.findall(r"@load\s+\w+\s*=", demo_text))
        if name == "{name}Loading/{name}Error/{name}Data/{name}Retry/{name}Reset":
            # 命名槽位（如 directLoading/historyError/pricesData/directRetry）——
            # LoadingBox/ErrorText 等 CSS 类名与 feedLoadingMore 不构成尾缀匹配
            return len(re.findall(r"\b\w+(?:Loading|Error|Data|Retry)\b", demo_text))
        if name == "reset（匿名）":
            # LOAD_RESERVED_NAMES 的 reset 槽位：命名实例形态 @onPress={directReset}
            # 等（匿名 reset 无新 demo——LoadDemo 维持不变，见 spec §8.2）
            return len(re.findall(r"@onPress=\{\w+Reset\}", demo_text))
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

# ── 批次 4 §5.4 断言：DataFetching 迁移后页面仍在 + 新 demo 页接线 ──
failures = []

def strip_comments(s: str) -> str:
    return re.sub(r"//[^\n]*", "", s)

df = strip_comments((DEMO / "pages" / "DataFetching.omni").read_text(encoding="utf-8"))
if len(re.findall(r"@load\s+\w+\s*=", df)) < 2:
    failures.append("DataFetching.omni 缺少 §8.1 双例 @load（direct/api 命名实例）")
for fn in ("loadDirect", "loadViaApi"):
    if fn not in df:
        failures.append(f"DataFetching.omni 缺少 loader {fn}")
for btn in ("directRetry", "apiRetry", "directReset", "apiReset"):
    if f"@onPress={{{btn}}}" not in df:
        failures.append(f"DataFetching.omni 缺少按钮 {btn}")
if re.search(r"\bsignal\s+(status|resultText|errorMsg)\b", df):
    failures.append("DataFetching.omni 残留旧形态手写状态机 signal（ADR-075 迁移纪律）")

ui = (DEMO / "pages" / "UiComponents.omni").read_text(encoding="utf-8")
for comp in ("PaginationDemo", "PollingDemo"):
    if f"<{comp} " not in ui:
        failures.append(f"UiComponents.omni 未接线 <{comp} />")
for rel in ("components/PaginationDemo.omni", "components/PollingDemo.omni"):
    if not (DEMO / rel).exists():
        failures.append(f"缺失新 demo 文件 {rel}")

if failures:
    print("\n" + "=" * 78)
    print(f"批次 4 §5.4 断言失败（{len(failures)}）:")
    for f in failures:
        print(f"  ❌ {f}")
    sys.exit(1)
print("\n批次 4 §5.4 断言通过：DataFetching 迁移形态 + Pagination/Polling 接线确认")
