// components/design/CodeDrawer.tsx
// Shared code drawer — slides up from bottom to show generated .omni source.
// Used globally across all tab types (design + component).

interface CodeDrawerProps {
  source: string;
  fileName: string;
  onClose: () => void;
}

const DRAWER_HEIGHT = 260;

export function CodeDrawer({ source, fileName, onClose }: CodeDrawerProps) {
  return (
    <div style={{
      height: DRAWER_HEIGHT,
      minHeight: 180,
      borderTop: "2px solid var(--dp-accent, #6366f1)",
      display: "flex",
      flexDirection: "column",
      background: "var(--dp-surface, #fff)",
      flexShrink: 0,
    }}>
      {/* Header */}
      <div style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "6px 16px",
        background: "var(--dp-bg, #f7f6fa)",
        borderBottom: "1px solid var(--dp-border, #e8e5f2)",
        flexShrink: 0,
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: "var(--dp-text-secondary, #6b6893)", display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ color: "var(--dp-accent, #6366f1)" }}>{"</>"}</span>
          {fileName}
        </span>
        <button type="button"
          onClick={onClose}
          style={{
            border: "none", background: "none", cursor: "pointer",
            fontSize: 18, color: "var(--dp-text-muted, #a09db8)",
            padding: "0 4px", lineHeight: 1, borderRadius: 4,
          }}
          title="Close code panel"
        >×</button>
      </div>
      {/* Body */}
      <pre style={{
        fontFamily: "'Cascadia Code', 'Fira Code', monospace",
        fontSize: 13, lineHeight: 1.6,
        color: "var(--dp-text, #1e1b4b)",
        padding: "12px 16px", overflow: "auto",
        whiteSpace: "pre-wrap", flex: 1, margin: 0, tabSize: 2,
      }}>
        {source}
      </pre>
    </div>
  );
}
