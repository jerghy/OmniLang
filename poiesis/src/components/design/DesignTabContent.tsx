import { useState, useMemo, useCallback, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import { parseOmniSource } from "../../services/codegen";
import { api } from "../../commands";
import { TokenEditor } from "./TokenEditor";
import { ClassEditor } from "./ClassEditor";
import { VariantEditor } from "./VariantEditor";
import { type TokenType, tokenTypeToCategory } from "../../core/types/design";

type DesignView = "tokens" | "classes" | "variants";

const TAB_BTN: React.CSSProperties = {
  padding: "8px 20px",
  background: "none",
  border: "none",
  borderBottomWidth: "2px",
  borderBottomStyle: "solid",
  borderBottomColor: "transparent",
  color: "#64748b",
  fontSize: 13,
  fontWeight: 500,
  cursor: "pointer",
  fontFamily: "inherit",
};

const TAB_BTN_ACTIVE: React.CSSProperties = {
  ...TAB_BTN,
  borderBottomColor: "#6366f1",
  color: "#6366f1",
  fontWeight: 600,
};

/** Height of the code drawer when open (percentage of available height). */
const CODE_DRAWER_HEIGHT = "38%";

export function DesignTabContent() {
  const [view, setView] = useState<DesignView>("tokens");
  const [showCode, setShowCode] = useState(false);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);

  // ── Load design.omni content from disk on mount ──
  useEffect(() => {
    const activeTabId = usePoiesisStore.getState().designPage.activeTabId;
    if (!activeTabId) return;
    const tab = usePoiesisStore.getState().designPage.openTabs.find((t) => t.id === activeTabId);
    if (!tab || tab.tabType !== "design") return;
    api.project.readFile(tab.filePath).then((res) => {
      if (!res.content) return;
      const result = parseOmniSource(res.content);
      if ((result.designTokens?.length ?? 0) > 0
          || (result.designClasses?.length ?? 0) > 0
          || (result.designVariants?.length ?? 0) > 0) {
        const { loadDesign } = usePoiesisStore.getState();
        loadDesign({
          name: result.designName || "AppDesign",
          tokens: (result.designTokens ?? []).map((t) => ({
            id: t.id,
            name: t.name,
            category: tokenTypeToCategory(t.tokenType),
            tokenType: t.tokenType as TokenType,
            value: t.value,
          })),
          classes: (result.designClasses ?? []).map((c) => ({
            id: c.id,
            name: c.name,
            properties: [],
            pseudoStates: [],
          })),
          variants: (result.designVariants ?? []).map((v) => ({
            id: v.id,
            name: v.name,
            dimensions: [],
            defaults: {},
            baseProperties: [],
            branches: [],
          })),
        });
      }
    }).catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const designSource = useMemo(() => emitDesignSource(activeDesign), [activeDesign]);

  const handleToggleCode = useCallback(() => {
    setShowCode((v) => !v);
  }, []);

  const handleCloseCode = useCallback(() => {
    setShowCode(false);
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "#f8fafc" }}>
      {/* ═══ Outer layer header: main view tabs (Tokens | Classes | Variants) ═══ */}
      <div style={{
        display: "flex",
        gap: 0,
        borderBottom: "1px solid #e2e8f0",
        padding: "0 24px",
        alignItems: "center",
        background: "#fff",
        minHeight: 42,
      }}>
        {(["tokens", "classes", "variants"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            style={view === v ? TAB_BTN_ACTIVE : TAB_BTN}
          >
            {v === "tokens" ? "Tokens" : v === "classes" ? "Classes" : "Variants"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        {/* Inner layer: Code toggle — subtle, inside the outer header */}
        <button
          onClick={handleToggleCode}
          title={showCode ? "Hide generated code" : "Show generated code"}
          style={{
            padding: "4px 12px",
            border: showCode ? "1px solid #6366f1" : "1px solid #e2e8f0",
            borderRadius: 6,
            background: showCode ? "#6366f1" : "#f8fafc",
            color: showCode ? "#fff" : "#64748b",
            cursor: "pointer",
            fontSize: 12,
            fontWeight: showCode ? 600 : 400,
            fontFamily: "inherit",
            transition: "all 150ms",
          }}
        >
          {showCode ? "Code ▾" : "Code ▸"}
        </button>
      </div>

      {/* ═══ Content area: outer layer (editor) + inner layer (code drawer) ═══ */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minHeight: 0 }}>
        {/* Outer layer: design editor — full width, always visible */}
        <div
          style={{
            flex: 1,
            overflow: "auto",
            padding: "24px 28px",
            minHeight: 0,
          }}
        >
          {view === "tokens" && <TokenEditor />}
          {view === "classes" && <ClassEditor />}
          {view === "variants" && <VariantEditor />}
        </div>

        {/* Inner layer: code drawer — slides up from bottom when toggled */}
        {showCode && (
          <div style={{
            height: CODE_DRAWER_HEIGHT,
            minHeight: 180,
            borderTop: "2px solid #6366f1",
            display: "flex",
            flexDirection: "column",
            background: "#fff",
            flexShrink: 0,
          }}>
            {/* Drawer header */}
            <div style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "6px 16px",
              background: "#f1f5f9",
              borderBottom: "1px solid #e2e8f0",
              flexShrink: 0,
            }}>
              <span style={{ fontSize: 12, fontWeight: 600, color: "#475569", display: "flex", alignItems: "center", gap: 6 }}>
                <span style={{ color: "#6366f1" }}>{"</>"}</span>
                design.omni
              </span>
              <button
                onClick={handleCloseCode}
                style={{
                  border: "none",
                  background: "none",
                  cursor: "pointer",
                  fontSize: 18,
                  color: "#94a3b8",
                  padding: "0 4px",
                  lineHeight: 1,
                  borderRadius: 4,
                }}
                title="Close code panel"
              >
                ×
              </button>
            </div>
            {/* Drawer body */}
            <pre style={{
              fontFamily: "'Cascadia Code', 'Fira Code', monospace",
              fontSize: 13,
              lineHeight: 1.6,
              color: "#1e293b",
              padding: "12px 16px",
              overflow: "auto",
              whiteSpace: "pre-wrap",
              flex: 1,
              margin: 0,
              tabSize: 2,
            }}>
              {designSource}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
