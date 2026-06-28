/**
 * 代码所有权冲突解决面板。
 *
 * 当画布修改与 @editor:hand-written 手写区域重叠时弹出，
 * 让用户选择保留哪个版本或手动合并。
 */


export interface ConflictInfo {
  managedContent: string;
  handwrittenContent: string;
  lineRange: string;
  regionType: string;  // "managed" | "handwritten" — used in title display
}

interface ConflictPanelProps {
  conflict: ConflictInfo;
  onResolve: (choice: "keep-managed" | "keep-handwritten" | "merge") => void;
  onDismiss: () => void;
}

export function ConflictPanel({ conflict, onResolve, onDismiss }: ConflictPanelProps) {
  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: "rgba(0,0,0,0.4)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
      onClick={onDismiss}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: "#FFF",
          borderRadius: 12,
          boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
          maxWidth: 720,
          width: "90%",
          maxHeight: "85vh",
          overflow: "auto",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid #F3F4F6",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 20 }}>⚡</span>
            <div>
              <h3 style={{ margin: 0, fontSize: 15, color: "#92400E" }}>
                编辑冲突 ({conflict.regionType === "handwritten" ? "手写区域" : "托管区域"})
              </h3>
              <p style={{ margin: "2px 0 0", fontSize: 12, color: "#92400E" }}>
                画布修改（{conflict.lineRange}）与手写代码区域重叠
              </p>
            </div>
          </div>
          <button
            onClick={onDismiss}
            style={{
              background: "none",
              border: "none",
              fontSize: 18,
              color: "#9CA3AF",
              cursor: "pointer",
            }}
          >
            ✕
          </button>
        </div>

        {/* Side-by-side diff */}
        <div style={{ display: "flex", gap: 0 }}>
          {/* Managed version */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                padding: "6px 12px",
                fontSize: 11,
                fontWeight: 600,
                color: "#6B7280",
                background: "#F9FAFB",
                borderBottom: "1px solid #E5E7EB",
              }}
            >
              🎨 画布版本 (managed)
            </div>
            <pre
              style={{
                margin: 0,
                padding: 12,
                fontSize: 12,
                background: "#F0FDF4",
                overflow: "auto",
                maxHeight: 240,
                whiteSpace: "pre-wrap",
                fontFamily: "'Cascadia Code', 'Fira Code', monospace",
                color: "#065F46",
                borderRight: "1px solid #E5E7EB",
              }}
            >
              {conflict.managedContent || "(空)"}
            </pre>
          </div>

          {/* Handwritten version */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                padding: "6px 12px",
                fontSize: 11,
                fontWeight: 600,
                color: "#6B7280",
                background: "#F9FAFB",
                borderBottom: "1px solid #E5E7EB",
              }}
            >
              ✍️ 手写版本 (hand-written)
            </div>
            <pre
              style={{
                margin: 0,
                padding: 12,
                fontSize: 12,
                background: "#FEF2F2",
                overflow: "auto",
                maxHeight: 240,
                whiteSpace: "pre-wrap",
                fontFamily: "'Cascadia Code', 'Fira Code', monospace",
                color: "#991B1B",
              }}
            >
              {conflict.handwrittenContent || "(空)"}
            </pre>
          </div>
        </div>

        {/* Actions */}
        <div
          style={{
            padding: "16px 20px",
            borderTop: "1px solid #F3F4F6",
            display: "flex",
            gap: 8,
            justifyContent: "flex-end",
          }}
        >
          <button
            onClick={() => onResolve("keep-handwritten")}
            style={{
              padding: "8px 16px",
              borderRadius: 6,
              border: "1px solid #D1D5DB",
              background: "#FFF",
              cursor: "pointer",
              fontSize: 13,
              color: "#374151",
            }}
          >
            ✍️ 保留手写版本
          </button>
          <button
            onClick={() => onResolve("keep-managed")}
            style={{
              padding: "8px 16px",
              borderRadius: 6,
              border: "1px solid #D1D5DB",
              background: "#FFF",
              cursor: "pointer",
              fontSize: 13,
              color: "#374151",
            }}
          >
            🎨 保留画布版本
          </button>
          <button
            onClick={() => onResolve("merge")}
            style={{
              padding: "8px 20px",
              borderRadius: 6,
              border: "none",
              background: "#4F46E5",
              color: "#FFF",
              cursor: "pointer",
              fontSize: 13,
              fontWeight: 600,
            }}
          >
            🔀 手动合并
          </button>
        </div>
      </div>
    </div>
  );
}
