// poiesis/src/components/design/BehaviorTab.tsx
// [行为] 标签：M3 行为卡（当[事件]→[动作][参数]，T1 提交为 no-op stub）+
// 现有事件编辑（EventsSection）保功能（T2 换 applyBehaviorCard 接线）。
import { useElements } from "../../store/hooks";
import { useHandlerItems } from "../panels/inspector/shared";
import { EventsSection } from "../panels/inspector/EventsSection";
import { BehaviorCard } from "./BehaviorCard";

export function BehaviorTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const handlerItems = useHandlerItems();
  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以编辑行为</div>;
  if (!("tag" in el)) return null;
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <BehaviorCard
        elementId={el.id}
        onCommit={() => {
          /* T2 wires applyBehaviorCard */
        }}
      />
      <EventsSection element={el} handlerItems={handlerItems} />
    </div>
  );
}
