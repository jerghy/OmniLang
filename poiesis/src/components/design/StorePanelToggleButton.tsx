// poiesis/src/components/design/StorePanelToggleButton.tsx
// 📦 Store 抽屉开关（原 ContextInspector 标题栏按钮，右栏删除后搬入工具栏）。
import { usePoiesisStore } from "../../store";

export function StorePanelToggleButton() {
  const visible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const toggle = usePoiesisStore((s) => s.setStorePanelVisible);
  return (
    <button
      type="button"
      data-testid="store-toggle"
      title={visible ? "关闭 Store 面板" : "打开 Store 面板"}
      onClick={() => toggle(!visible)}
      style={{
        border: visible ? "1px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
        background: visible ? "var(--ol-color-primary-subtle)" : "transparent",
        color: visible ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
        borderRadius: 4,
        padding: "2px 8px",
        fontSize: 12,
        cursor: "pointer",
      }}
    >
      📦 Store
    </button>
  );
}
