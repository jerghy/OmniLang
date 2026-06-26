/**
 * Read-only indicator — compact floating button at bottom-left.
 * Shown as info-only now; hand-written .omni files can be edited
 * directly since annotations live inline in the source.
 */

import React from "react";

interface ReadOnlyBannerProps {
  warnings: string[];
  source: string;
  onConverted?: () => void;
}

export function ReadOnlyBanner({ warnings }: ReadOnlyBannerProps): React.ReactElement {
  const [expanded, setExpanded] = React.useState(false);

  return (
    <div
      style={{
        position: "fixed",
        bottom: 28,
        left: 12,
        zIndex: 9999,
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-start",
        gap: 6,
      }}
    >
      {/* Expandable warning/error details */}
      {expanded && (
        <div
          style={{
            background: "#FFF9DB",
            border: "1px solid #FFC107",
            borderRadius: 8,
            padding: "8px 12px",
            fontSize: 11,
            maxWidth: 260,
            boxShadow: "0 4px 12px rgba(0,0,0,0.12)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>🔒 Read-only Mode</div>
          <div style={{ color: "#856404" }}>
            This file was hand-written. The editor cannot write to it.
          </div>
          {warnings.length > 0 && (
            <ul style={{ margin: "4px 0 0 0", paddingLeft: 14, color: "#856404" }}>
              {warnings.map((w, i) => (
                <li key={i}>{w}</li>
              ))}
            </ul>
          )}
          {error && <div style={{ color: "#DC2626", marginTop: 4 }}>{error}</div>}
        </div>
      )}

      {/* Compact button row */}
      <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
        <button
          onClick={handleConvert}
          disabled={converting}
          title="Convert hand-written .omni to visual project"
          style={{
            background: converting ? "#FCD34D" : "#F59E0B",
            color: "#FFF",
            border: "none",
            borderRadius: 6,
            padding: "5px 12px",
            fontSize: 11,
            fontWeight: 600,
            cursor: converting ? "not-allowed" : "pointer",
            whiteSpace: "nowrap",
            boxShadow: "0 2px 6px rgba(0,0,0,0.15)",
          }}
        >
          {converting ? "⏳ Converting..." : "🔧 Convert to Visual"}
        </button>
        {(warnings.length > 0 || error) && (
          <button
            onClick={() => setExpanded(!expanded)}
            title="Show warnings"
            style={{
              background: expanded ? "#FFF9DB" : "#F3F4F6",
              color: expanded ? "#92400E" : "#6B7280",
              border: "1px solid #E5E7EB",
              borderRadius: 4,
              padding: "3px 7px",
              fontSize: 10,
              fontWeight: 600,
              cursor: "pointer",
              whiteSpace: "nowrap",
            }}
          >
            {warnings.length > 0 ? `⚠ ${warnings.length}` : "!"}
          </button>
        )}
      </div>
    </div>
  );
}
