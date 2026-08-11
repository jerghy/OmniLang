// poiesis/src/components/design/ContentTab.tsx
// [内容] 标签 = 选中元素的接口（§15.5）：槽位清单（只读）+ 已绑来源印记；BindingsSection 保现有绑定编辑（M3 换点名绑定）。
import { useElements } from "../../store/hooks";
import { useSignalItems } from "../panels/inspector/shared";
import { BindingsSection } from "../panels/inspector/BindingsSection";

// 按元素类型计算接口槽位（M1 只读展示；形状/接线交互 M3）
function slotsFor(el: { tag?: string; nodeType?: string }): Array<{ key: string; label: string; kind: "文本" | "输入" | "列表" | "条件" | "属性" }> {
  if (el.nodeType === "for") return [{ key: "list", label: "列表内容", kind: "列表" }];
  if (el.nodeType === "conditional") return [{ key: "if", label: "显示条件", kind: "条件" }];
  const tag = el.tag ?? "";
  if (["input", "textarea", "select"].includes(tag)) return [{ key: "value", label: "值", kind: "输入" }];
  if (tag === "img") return [{ key: "src", label: "图片地址", kind: "属性" }];
  if (tag === "a") return [{ key: "href", label: "跳转地址", kind: "属性" }];
  return [{ key: "text", label: "文本内容", kind: "文本" }];
}

export function ContentTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const signalItems = useSignalItems();
  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以查看接口</div>;

  const slots = slotsFor(el);
  // 来源印记展示（spec M1 明列，审查修正）：已绑槽位显示"已绑定"徽章——来源名徽章
  // （"来自 X"）与线显示在 M3 同做，M1 以存在性标记 + BindingsSection 实际绑定内容兑现。
  // 绑定字段形状以 ViewElementData.bindings 实际定义为准（实现时按真实字段读取）。
  const bindings = ("bindings" in el && el.bindings) as Record<string, unknown> | undefined;
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <h3 style={{ fontSize: 13, fontWeight: 600, margin: "0 0 8px", color: "var(--ol-gray-700)" }}>接口</h3>
      {slots.map((s) => {
        const bound = bindings != null && Object.keys(bindings).some((k) => k === s.key || k.includes(s.key));
        return (
          <div key={s.key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "6px 8px", borderBottom: "1px solid var(--ol-border)" }}>
            <span style={{ fontSize: 12 }}>{s.label}</span>
            <span style={{ fontSize: 11, display: "flex", gap: 6, alignItems: "center" }}>
              {bound && (
                <em data-testid={`slot-bound-${s.key}`} style={{ fontStyle: "normal", color: "var(--ol-color-primary)", background: "var(--ol-color-primary-subtle)", borderRadius: 999, padding: "1px 8px" }}>
                  已绑定
                </em>
              )}
              {s.kind}
            </span>
          </div>
        );
      })}
      <div style={{ marginTop: 12, fontSize: 11, color: "var(--ol-text-muted)" }}>
        绑定编辑（M3 换点名绑定 UI；现有编辑暂沿用）
      </div>
      {"tag" in el && <BindingsSection element={el} signalItems={signalItems} />}
    </div>
  );
}
