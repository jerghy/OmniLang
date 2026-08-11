// poiesis/src/components/design/BehaviorTab.tsx
// [行为] 标签：现有事件编辑（EventsSection）保功能；M3 换行为卡（当[点击]→[把][计数器][加1]）。
import { useElements } from "../../store/hooks";
import { useHandlerItems } from "../panels/inspector/shared";
import { EventsSection } from "../panels/inspector/EventsSection";

export function BehaviorTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const handlerItems = useHandlerItems();
  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以编辑行为</div>;
  if (!("tag" in el)) return null;
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <EventsSection element={el} handlerItems={handlerItems} />
    </div>
  );
}
