import { usePoiesisStore } from "../../store";
import styles from "./ArchitectureToolbar.module.css";

interface ArchitectureToolbarProps {
  viewMode: "preview" | "graph";
  onViewModeChange: (mode: "preview" | "graph") => void;
}

export function ArchitectureToolbar({
  viewMode,
  onViewModeChange,
}: ArchitectureToolbarProps) {
  const mode = usePoiesisStore((s) => s.architecture.mode);
  const focusComponentId = usePoiesisStore(
    (s) => s.architecture.focusComponentId
  );

  const modeLabel =
    mode === "overall"
      ? "整体架构"
      : `局部架构: ${focusComponentId ?? "unknown"}`;

  return (
    <div className={styles.toolbar}>
      {/* Left: Logo */}
      <div
        style={{
          fontSize: 16,
          fontWeight: 700,
          color: "var(--ol-color-primary)",
          letterSpacing: -0.5,
        }}
      >
        Poiesis
      </div>

      {/* Center: Mode Toggle */}
      <div
        style={{
          display: "flex",
          gap: 0,
          background: "var(--ol-surface-hover)",
          borderRadius: 8,
          padding: 3,
        }}
      >
        <ToggleButton
          active={viewMode === "preview"}
          onClick={() => onViewModeChange("preview")}
        >
          📐 界面预览
        </ToggleButton>
        <ToggleButton
          active={viewMode === "graph"}
          onClick={() => onViewModeChange("graph")}
        >
          🔗 关系图谱
        </ToggleButton>
      </div>

      {/* Right: Mode Badge */}
      <div
        style={{
          fontSize: 12,
          color: "var(--ol-text-secondary)",
          background: "var(--ol-gray-50)",
          padding: "4px 12px",
          borderRadius: 12,
          border: "1px solid var(--ol-border)",
        }}
      >
        {modeLabel}
      </div>
    </div>
  );
}

function ToggleButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button type="button"
      onClick={onClick}
      style={{
        padding: "6px 16px",
        fontSize: 13,
        fontWeight: active ? 600 : 400,
        color: active ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
        background: active ? "var(--ol-surface)" : "transparent",
        border: "none",
        borderRadius: 6,
        cursor: "pointer",
        boxShadow: active ? "0 1px 3px rgba(0,0,0,0.1)" : "none",
        transition: "border-color 150ms ease, background-color 150ms ease, opacity 150ms ease",
      }}
    >
      {children}
    </button>
  );
}
