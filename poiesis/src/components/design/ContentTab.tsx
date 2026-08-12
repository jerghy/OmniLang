// poiesis/src/components/design/ContentTab.tsx
// [内容] 标签 = 选中元素的接口（§15.5）：槽位清单 + 已绑来源印记。
// M3 T3 点名绑定：点槽 → BindingPicker（信号 / fn 出口 / 画布元素）→ 选择落盘现有语法
// （文本/属性 → label/src/href 插值；输入 → bindings；列表 → for.collection；条件 → if 分支）。
// BindingsSection 保留为既有绑定编辑（输入类槽位沿用）。
import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { useElements } from "../../store/hooks";
import { useSignalItems } from "../panels/inspector/shared";
import { BindingsSection } from "../panels/inspector/BindingsSection";
import { BindingPicker, type BindingSource } from "./BindingPicker";
import { buildBindingSources } from "./binding-sources";
import type { ElementData } from "../../core/types/element";

interface SlotDef {
  key: string;
  label: string;
  kind: "文本" | "输入" | "列表" | "条件" | "属性";
}

// 按元素类型计算接口槽位（M1 只读展示；M3 点名绑定）
function slotsFor(el: { tag?: string; nodeType?: string }): SlotDef[] {
  if (el.nodeType === "for") return [{ key: "list", label: "列表内容", kind: "列表" }];
  if (el.nodeType === "conditional") return [{ key: "if", label: "显示条件", kind: "条件" }];
  const tag = el.tag ?? "";
  if (["input", "textarea", "select"].includes(tag)) return [{ key: "value", label: "值", kind: "输入" }];
  if (tag === "img") return [{ key: "src", label: "图片地址", kind: "属性" }];
  if (tag === "a") return [{ key: "href", label: "跳转地址", kind: "属性" }];
  // 文本槽键对齐落盘键 label（props.label 存文本内容）
  return [{ key: "label", label: "文本内容", kind: "文本" }];
}

/** @if 分支下标统一来源（badge 检查与 pick 落盘共用；非条件块返回 -1） */
function ifBranchIndex(el: ElementData): number {
  if (el.nodeType !== "conditional") return -1;
  return el.branches.findIndex((b) => b.type === "if");
}

/** 槽位是否已绑：bindings 键精确匹配（M1）∪ 插值/列表/条件写法（M3 落盘路径） */
function slotBound(el: ElementData, slot: SlotDef): boolean {
  return slotSource(el, slot) != null;
}

/** 槽位绑定来源名（M3 T6 来源印记）：bindings 值 → 插值表达式 → 集合/条件表达式 */
function slotSource(el: ElementData, slot: SlotDef): string | null {
  if ("bindings" in el && el.bindings != null && el.bindings[slot.key] != null) {
    const t = el.bindings[slot.key].trim();
    return t.length > 0 ? t : null;
  }
  if (el.nodeType === "for" && slot.key === "list") {
    const c = (el.collection ?? "").trim();
    return c.length > 0 ? c : null;
  }
  if (el.nodeType === "conditional" && slot.key === "if") {
    const idx = ifBranchIndex(el);
    const cond = idx >= 0 ? (el.branches[idx].condition ?? "").trim() : "";
    return cond.length > 0 ? cond : null;
  }
  if ("props" in el) {
    const v = el.props?.[slot.key];
    if (typeof v === "string") {
      const m = v.match(/\{([^}]*)\}/);
      if (m) {
        const t = m[1].trim();
        if (t) return t; // 空插值 { } → 假亮回归：取到空来源名视为未绑
      }
    }
  }
  return null;
}

export function ContentTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const signalItems = useSignalItems();
  const [pickingFor, setPickingFor] = useState<string | null>(null); // 打开的槽位 key

  // M3 T3 来源组装（SourceNodeLayer 同源选择器）：组件/store 信号 + fn 出口 + 画布元素
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);
  const debugValues = usePoiesisStore((s) => s.debugValues);
  const setProp = usePoiesisStore((s) => s.setProp);
  const setElementBinding = usePoiesisStore((s) => s.setElementBinding);
  const updateForBlock = usePoiesisStore((s) => s.updateForBlock);
  const updateBranchCondition = usePoiesisStore((s) => s.updateBranchCondition);

  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const componentName = activeTab?.tabType === "component" ? (activeTab.componentName ?? null) : null;

  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以查看接口</div>;

  const slots = slotsFor(el);
  const sources = buildBindingSources({
    componentName,
    componentDefs,
    storeDefs,
    inlineStoreDefs,
    storeReferences,
    debugValues,
    elements,
  });

  // 点名绑定落盘：按槽位种类写现有语法（文本/属性槽键即落盘 prop 键）
  const pick = (slot: SlotDef, source: BindingSource) => {
    switch (slot.kind) {
      case "文本":
      case "属性":
        setProp(el.id, slot.key, `{${source.name}}`);
        break;
      case "输入":
        setElementBinding(el.id, slot.key, source.name);
        break;
      case "列表":
        updateForBlock(el.id, { collection: source.name });
        break;
      case "条件": {
        const idx = ifBranchIndex(el);
        updateBranchCondition(el.id, idx >= 0 ? idx : 0, source.name);
        break;
      }
    }
    setPickingFor(null);
  };

  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <h3 style={{ fontSize: 13, fontWeight: 600, margin: "0 0 8px", color: "var(--ol-gray-700)" }}>接口</h3>
      {slots.map((s) => {
        const bound = slotBound(el, s);
        const source = bound ? slotSource(el, s) : null;
        return (
          <div
            key={s.key}
            style={{ position: "relative", display: "flex", justifyContent: "space-between", alignItems: "center", padding: "6px 8px", borderBottom: "1px solid var(--ol-border)" }}
          >
            <span style={{ fontSize: 12 }}>{s.label}</span>
            <span style={{ fontSize: 11, display: "flex", gap: 6, alignItems: "center" }}>
              {bound && (
                <em
                  data-testid={`slot-bound-${s.key}`}
                  title={source ?? undefined}
                  style={{ fontStyle: "normal", color: "var(--ol-color-primary)", background: "var(--ol-color-primary-subtle)", borderRadius: 999, padding: "1px 8px", maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                >
                  来自 {source}
                </em>
              )}
              {s.kind}
              <button
                type="button"
                data-testid={`slot-bind-${s.key}`}
                onClick={() => setPickingFor(pickingFor === s.key ? null : s.key)}
                style={{ fontSize: 11, padding: "1px 8px", borderRadius: 6, border: "1px solid var(--ol-border)", background: "var(--ol-surface-raised)", color: "var(--ol-gray-700)", cursor: "pointer" }}
              >
                {bound ? "替换" : "绑定"}
              </button>
              {pickingFor === s.key && <BindingPicker sources={sources} onPick={(src) => pick(s, src)} onClose={() => setPickingFor(null)} />}
            </span>
          </div>
        );
      })}
      <div style={{ marginTop: 12, fontSize: 11, color: "var(--ol-text-muted)" }}>
        点槽绑定来源（信号 / fn 出口 / 画布元素）；输入类槽位可继续用下方绑定编辑
      </div>
      {"tag" in el && <BindingsSection element={el} signalItems={signalItems} />}
    </div>
  );
}
