// poiesis/src/components/design/StructureTab.tsx
// [结构] 标签：原 ContextInspector 的属性编辑体（迁移，不含面板外壳）。
// 事件（[行为]）/绑定（[内容]）分家——EventsSection/BindingsSection 分别在 BehaviorTab/ContentTab。
// 计划原文 import { useMemo }——正文未用且 tsconfig noUnusedLocals 开启，按实现裁剪。
import { useElements } from "../../store/hooks";
import { BASIC_TAGS, useSignalItems } from "../panels/inspector/shared";
import { PropsSection } from "../panels/inspector/PropsSection";
import { ClassesSection } from "../panels/inspector/ClassesSection";
import { VariantBindingSection } from "../panels/inspector/VariantBindingSection";
import { InlineStylePanel } from "../panels/inspector/InlineStylePanel";
import { ConditionalBlockEditor } from "../panels/inspector/ConditionalBlockEditor";
import { ForBlockEditor } from "../panels/inspector/ForBlockEditor";
import { BranchConditionEditor } from "../panels/inspector/BranchConditionEditor";

export function StructureTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const signalItems = useSignalItems();
  const el = elements.find((e) => e.id === elementId);

  // 分支合成 id：comp.@if.0.br1
  const branchMatch = elementId.match(/^(.+)\.br(\d+)$/);
  const condBlock = branchMatch
    ? elements.find((e) => e.id === branchMatch[1] && e.nodeType === "conditional")
    : null;

  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以编辑属性</div>;

  if (el.nodeType === "for") return <ForBlockEditor blockId={el.id} />;
  if (el.nodeType === "conditional") {
    return <div style={{ padding: 12, overflow: "auto" }}><ConditionalBlockEditor blockId={el.id} /></div>;
  }
  if (branchMatch && condBlock) {
    return <div style={{ padding: 12, overflow: "auto" }}><BranchConditionEditor blockId={branchMatch[1]} branchIndex={parseInt(branchMatch[2], 10)} /></div>;
  }

  const isBasic = "tag" in el && BASIC_TAGS.has(el.tag);
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <h3 style={{ fontSize: 13, fontWeight: 600, margin: "0 0 8px", color: "var(--ol-gray-700)" }}>
        {"tag" in el ? el.tag : "unknown"}
        {"componentRef" in el && el.componentRef ? ` (${el.componentRef})` : ""}
      </h3>
      <PropsSection element={el} signalItems={signalItems} />
      {isBasic && <ClassesSection element={el} />}
      {isBasic && <VariantBindingSection element={el} />}
      {isBasic && <InlineStylePanel element={el} />}
    </div>
  );
}
