import { useCallback, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { parseOmniSource } from "../../services/codegen";
import { parseStyleValue } from "../../services/codegen/design-value-parser";
import { uid } from "../../core/utils/id";
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
  color: "var(--dp-text-muted, #a09db8)",
  fontSize: 13,
  fontWeight: 500,
  cursor: "pointer",
  fontFamily: "inherit",
  transition: "color 150ms ease, border-color 150ms ease",
};

const TAB_BTN_ACTIVE: React.CSSProperties = {
  ...TAB_BTN,
  borderBottomColor: "var(--dp-accent, #6366f1)",
  color: "var(--dp-accent, #6366f1)",
  fontWeight: 600,
};

export function DesignTabContent() {
  const designView = usePoiesisStore((s) => s.designPage.designView);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);

  const setDesignView = useCallback((v: DesignView) => {
    usePoiesisStore.setState((s) => ({
      designPage: { ...s.designPage, designView: v },
    }));
  }, []);

  // ── Load design.omni content from disk on mount (fallback only) ──
  // Design data is normally loaded during project init. Only re-parse from
  // disk if the store is empty (e.g. user opened design.omni without a project).
  useEffect(() => {
    const store = usePoiesisStore.getState();
    // Skip if design data already loaded (preserves unsaved edits across page switches)
    if (store.activeDesign.tokens.length > 0 || store.activeDesign.classes.length > 0) return;
    const tab = store.designPage.openTabs.find((t) => t.tabType === "design");
    if (!tab) return;
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
            properties: (c.properties ?? []).map((p) => ({
              id: uid("sp"),
              name: p.name,
              segments: parseStyleValue(p.value),
            })),
            pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
              id: uid("ps"),
              pseudo: ps.pseudo,
              properties: ps.properties.map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
            })),
          })),
          variants: (result.designVariants ?? []).map((v) => ({
            id: v.id,
            name: v.name,
            dimensions: (v.dimensions ?? []).map((d) => ({
              name: d.name,
              values: d.values.map((dv) => dv.name),
            })),
            defaults: v.defaults ?? {},
            baseProperties: (v.baseProperties ?? []).map((p) => ({
              id: uid("sp"),
              name: p.name,
              segments: parseStyleValue(p.value),
            })),
            branches: (v.branches ?? []).map((b) => {
              const dotIdx = b.name.indexOf(".");
              const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
              const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
              const props = b.properties.map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              }));
              return { id: uid("vb"), dimension: dim, value: val, properties: props };
            }),
          })),
        });
      }
    }).catch((e) => { console.warn("Design load failed", e); });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--dp-bg, #f7f6fa)" }}>
      {/* ═══ Outer layer header: main view tabs (Tokens | Classes | Variants) ═══ */}
      <div style={{
        display: "flex",
        gap: 0,
        borderBottom: "1px solid var(--dp-border, #e8e5f2)",
        padding: "0 16px",
        alignItems: "center",
        background: "var(--dp-surface, #fff)",
        minHeight: 42,
      }}>
        {(["tokens", "classes", "variants"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setDesignView(v)}
            style={designView === v ? TAB_BTN_ACTIVE : TAB_BTN}
          >
            {v === "tokens" ? "Tokens" : v === "classes" ? "Classes" : "Variants"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
      </div>

      {/* ═══ Content area: outer layer (editor) + inner layer (code drawer) ═══ */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minHeight: 0 }}>
        {/* Outer layer: design editor — full width, always visible */}
        <div
          style={{
            flex: 1,
            overflow: "auto",
            minHeight: 0,
          }}
        >
          {designView === "tokens" && <TokenEditor />}
          {designView === "classes" && <ClassEditor />}
          {designView === "variants" && <VariantEditor />}
        </div>

      </div>
    </div>
  );
}
