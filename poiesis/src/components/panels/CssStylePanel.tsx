// CssStylePanel — CSS style design panel for managing CSS variables and classes.
// Two sections: CSS variables (upper) and CSS classes (lower), each with add/edit/delete.
// Pseudo-classes (.btn:hover) are independent class cards.

import React, { useState, useMemo, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import type { CssVariable, StyleClass, StyleProperty } from "../../core/types/style";
import { uid } from "../../lib/utils/id";
import { DROPDOWN_PROPERTIES, PROPERTY_UNIT_MAP } from "../../lib/constants/style-properties";
import { FormulaValueInput } from "./css-panel/FormulaValueInput";

// ── Constants ──

const PREVIEW_TAGS = ["div", "p", "button", "h1", "h2", "h3", "span", "section", "input", "a"] as const;
type PreviewTag = typeof PREVIEW_TAGS[number];

const TAG_DISPLAY: Record<string, string> = {
  div: "<div>", p: "<p>", button: "<button>", h1: "<h1>", h2: "<h2>",
  h3: "<h3>", span: "<span>", section: "<section>", input: "<input>", a: "<a>",
};

const TAG_CONTENT: Record<string, string> = {
  div: "Box", p: "Text", button: "Button", h1: "Heading 1", h2: "Heading 2",
  h3: "Heading 3", span: "Inline", section: "Section", input: "Input", a: "Link",
};

// All CSS property names for the searchable dropdown
const ALL_CSS_PROPS = [
  ...Object.keys(DROPDOWN_PROPERTIES).sort(),
  ...Object.keys(PROPERTY_UNIT_MAP).filter((k) => !(k in DROPDOWN_PROPERTIES)).sort(),
];

// ── Shared Styles ──

const S = {
  panel: {
    display: "flex", flexDirection: "column", height: "100%",
    background: "#fff", overflow: "hidden",
  } as React.CSSProperties,

  header: {
    padding: "8px 12px", borderBottom: "1px solid #E2E8F0",
    fontSize: 12, fontWeight: 700, color: "#374151",
    display: "flex", alignItems: "center", gap: 6,
    userSelect: "none",
  } as React.CSSProperties,

  section: {
    borderBottom: "1px solid #E2E8F0",
  } as React.CSSProperties,

  sectionHeader: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "6px 12px", background: "#FAFAFA",
    borderBottom: "1px solid #F3F4F6",
    cursor: "pointer", userSelect: "none",
  } as React.CSSProperties,

  sectionTitle: {
    fontSize: 12, fontWeight: 700, color: "#6B7280",
    textTransform: "uppercase", display: "flex", alignItems: "center", gap: 4,
  } as React.CSSProperties,

  addBtn: {
    padding: "2px 8px", fontSize: 12, fontWeight: 600,
    border: "1px dashed #D1D5DB", borderRadius: 4,
    background: "transparent", color: "#6B7280", cursor: "pointer",
  } as React.CSSProperties,

  scrollArea: {
    flex: 1, overflow: "auto", padding: "4px 8px",
  } as React.CSSProperties,

  emptyHint: {
    padding: "16px 8px", textAlign: "center", fontSize: 12, color: "#9CA3AF",
  } as React.CSSProperties,

  rowLabel: {
    fontSize: 12, color: "#9CA3AF", fontFamily: "monospace",
    minWidth: 100, flexShrink: 0,
  } as React.CSSProperties,

  deleteBtn: {
    padding: "1px 4px", fontSize: 12, color: "#EF4444", cursor: "pointer",
    border: "none", background: "transparent", fontWeight: 700, lineHeight: 1,
  } as React.CSSProperties,
};

// ═══════════════════════════════════════════════════════════════════
// Main Panel
// ═══════════════════════════════════════════════════════════════════

export function CssStylePanel() {
  const componentName = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return null;
    return s.designPage.openTabs.find((t) => t.id === activeId)?.componentName ?? null;
  });

  const registry = usePoiesisStore((s) =>
    componentName ? s.styleRegistries[componentName] : undefined
  );

  const variables = registry?.variables ?? [];
  const classes = registry?.classes ?? [];

  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const toggleCollapse = (key: string) => setCollapsed((prev) => {
    const next = new Set(prev);
    next.has(key) ? next.delete(key) : next.add(key);
    return next;
  });

  if (!componentName) {
    return (
      <div style={S.panel}>
        <div style={S.header}>🎨 CSS 样式设计</div>
        <div style={S.emptyHint}>请先打开一个组件</div>
      </div>
    );
  }

  return (
    <div style={S.panel}>
      <div style={S.header}>🎨 CSS 样式设计</div>

      {/* ── Variables Section ── */}
      <div style={{ ...S.section, maxHeight: "40%", display: "flex", flexDirection: "column" }}>
        <div role="button" tabIndex={0} style={S.sectionHeader} onClick={() => toggleCollapse("vars")} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleCollapse("vars"); } }}>
          <span style={S.sectionTitle}>
            <span style={{ transform: collapsed.has("vars") ? "rotate(-90deg)" : "none", transition: "transform 0.15s", fontSize: 12 }}>▼</span>
            CSS 变量
            <span style={{ fontSize: 12, color: "#9CA3AF", fontWeight: 400 }}>({variables.length})</span>
          </span>
          <button type="button" style={S.addBtn} onClick={(e) => {
            e.stopPropagation();
            const store = usePoiesisStore.getState();
            const id = uid("var");
            store.addVariable(componentName, { id, name: "--new-variable", segments: [] });
            setCollapsed((prev) => { const next = new Set(prev); next.delete("vars"); return next; });
          }}>+ 新建</button>
        </div>
        {!collapsed.has("vars") && (
          <div style={{ overflow: "auto", flex: 1, padding: 4 }}>
            {variables.length === 0 ? (
              <div style={S.emptyHint}>暂无 CSS 变量</div>
            ) : (
              variables.map((v) => (
                <VariableRow key={v.id} variable={v} componentName={componentName} />
              ))
            )}
          </div>
        )}
      </div>

      {/* ── Classes Section ── */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <div role="button" tabIndex={0} style={S.sectionHeader} onClick={() => toggleCollapse("classes")} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleCollapse("classes"); } }}>
          <span style={S.sectionTitle}>
            <span style={{ transform: collapsed.has("classes") ? "rotate(-90deg)" : "none", transition: "transform 0.15s", fontSize: 12 }}>▼</span>
            CSS Class
            <span style={{ fontSize: 12, color: "#9CA3AF", fontWeight: 400 }}>({classes.length})</span>
          </span>
          <button type="button" style={S.addBtn} onClick={(e) => {
            e.stopPropagation();
            const store = usePoiesisStore.getState();
            const id = uid("cls");
            store.addClass(componentName, { id, name: "new-class", properties: [] });
            setCollapsed((prev) => { const next = new Set(prev); next.delete("classes"); return next; });
          }}>+ 新建</button>
        </div>
        {!collapsed.has("classes") && (
          <div style={S.scrollArea}>
            {classes.length === 0 ? (
              <div style={S.emptyHint}>暂无 CSS Class</div>
            ) : (
              classes.map((c) => (
                <ClassCard key={c.id} styleClass={c} componentName={componentName} variables={variables} />
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// VariableRow
// ═══════════════════════════════════════════════════════════════════

function VariableRow({ variable, componentName }: { variable: CssVariable; componentName: string }) {
  const [name, setName] = useState(variable.name);
  const [val, setVal] = useState(
    variable.segments.length > 0 && variable.segments[0].type === "text"
      ? variable.segments[0].value
      : ""
  );

  // Sync if external change
  if (variable.name !== name && variable.name !== "--new-variable") {
    // avoid resetting on first render; just keep local state
  }

  const commitName = () => {
    const n = name.startsWith("--") ? name : "--" + name;
    const clean = n.replace(/[^a-zA-Z0-9_-]/g, "-");
    usePoiesisStore.getState().updateVariable(componentName, variable.id, { name: clean });
    setName(clean);
  };

  const commitValue = () => {
    usePoiesisStore.getState().updateVariable(componentName, variable.id, {
      segments: val.trim() ? [{ type: "text", value: val.trim() }] : [],
    });
  };

  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: 4, padding: "3px 0", borderBottom: "1px solid #F9FAFB" }}>
      <input
        type="text" value={name} onChange={(e) => setName(e.target.value)}
        onBlur={commitName}
        placeholder="--name"
        aria-label="CSS variable name"
        style={{ width: 110, padding: "2px 4px", fontSize: 12, fontFamily: "monospace", border: "1px solid #E5E7EB", borderRadius: 3, color: "#7C3AED", background: "#FAFAFA" }}
        spellCheck={false}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <FormulaValueInput
          value={val}
          onChange={(v) => { setVal(v); }}
          onBlur={commitValue}
          variables={usePoiesisStore.getState().getRegistry(componentName).variables}
          placeholder="值，如 #6366f1, 16px"
        />
      </div>
      <button type="button" style={S.deleteBtn} onClick={() => usePoiesisStore.getState().removeVariable(componentName, variable.id)}>
        ×
      </button>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// ClassCard
// ═══════════════════════════════════════════════════════════════════

function ClassCard({ styleClass, componentName, variables }: {
  styleClass: StyleClass;
  componentName: string;
  variables: CssVariable[];
}) {
  const [expanded, setExpanded] = useState(true);
  const [editName, setEditName] = useState(styleClass.name);
  useEffect(() => { setEditName(styleClass.name); }, [styleClass.name]);
  const [previewTag, setPreviewTag] = useState<PreviewTag>("div");
  const [showPropSelector, setShowPropSelector] = useState(false);
  const [propSearch, setPropSearch] = useState("");

  const commitName = () => {
    const clean = editName.replace(/[^a-zA-Z0-9._:\-]/g, "-").replace(/^-+/, "");
    usePoiesisStore.getState().renameClass(componentName, styleClass.id, clean || "class");
    setEditName(clean || "class");
  };

  // Build preview style from properties
  const previewStyle = useMemo(() => {
    const s: Record<string, string> = {};
    for (const p of styleClass.properties) {
      if (p.segments.length > 0 && p.segments[0].type === "text") {
        s[p.name] = p.segments[0].value;
      }
    }
    return s;
  }, [styleClass.properties]);

  const filteredProps = ALL_CSS_PROPS.filter((p) =>
    p.toLowerCase().includes(propSearch.toLowerCase())
  );

  const addProperty = (propName: string) => {
    const id = uid("prop");
    usePoiesisStore.getState().addProperty(componentName, styleClass.id, {
      id, name: propName, segments: [],
    });
    setShowPropSelector(false);
    setPropSearch("");
  };

  const PreviewTag = previewTag as keyof React.JSX.IntrinsicElements;

  return (
    <div style={{
      border: "1px solid #E5E7EB", borderRadius: 4, marginBottom: 6,
      background: "#fff", overflow: "hidden",
    }}>
      {/* Card header */}
      <div role="button" tabIndex={0} style={{
        display: "flex", alignItems: "center", gap: 4, padding: "4px 6px",
        background: "#FAFAFA", borderBottom: expanded ? "1px solid #E5E7EB" : "none",
        cursor: "pointer",
      }} onClick={() => setExpanded(!expanded)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setExpanded(!expanded); } }}>
        <span style={{ fontSize: 12, transform: expanded ? "rotate(0deg)" : "rotate(-90deg)", transition: "transform 0.15s" }}>▼</span>
        <span style={{ fontSize: 12, color: "#9CA3AF", fontFamily: "monospace" }}>.</span>
        <input
          type="text" value={editName} onChange={(e) => setEditName(e.target.value)}
          onBlur={commitName}
          onClick={(e) => e.stopPropagation()}
          placeholder="class-name"
          aria-label="Class name"
          style={{
            flex: 1, border: "none", outline: "2px solid transparent", background: "transparent",
            fontSize: 12, fontFamily: "monospace", color: "#374151", fontWeight: 600,
            minWidth: 60,
          }}
          spellCheck={false}
        />
        <span style={{ fontSize: 12, color: "#9CA3AF" }}>{styleClass.properties.length} 属性</span>
        <button type="button" style={{ ...S.deleteBtn, fontSize: 14 }} onClick={(e) => {
          e.stopPropagation();
          usePoiesisStore.getState().removeClass(componentName, styleClass.id);
        }}>×</button>
      </div>

      {expanded && (
        <div style={{ padding: 6 }}>
          {/* Preview area */}
          <div style={{
            display: "flex", gap: 8, marginBottom: 8,
          }}>
            <div style={{
              flex: 1, minHeight: 50, border: "1px dashed #E2E8F0", borderRadius: 4,
              padding: 6, background: "#F8FAFC", display: "flex", alignItems: "center", justifyContent: "center",
              overflow: "hidden",
            }}>
              <PreviewTag style={{
                ...previewStyle,
                maxWidth: "100%", maxHeight: "100%",
                fontSize: previewStyle.fontSize || 11,
                overflow: "hidden",
              }}>
                {TAG_CONTENT[previewTag] || "Preview"}
              </PreviewTag>
            </div>
            <div style={{ width: 90, flexShrink: 0 }}>
              <select
                aria-label="Preview tag"
                value={previewTag}
                onChange={(e) => setPreviewTag(e.target.value as PreviewTag)}
                style={{
                  width: "100%", padding: "2px 4px", fontSize: 12,
                  border: "1px solid #E5E7EB", borderRadius: 3,
                  background: "#fff", color: "#374151",
                }}
              >
                {PREVIEW_TAGS.map((t) => (
                  <option key={t} value={t}>{TAG_DISPLAY[t]} {t}</option>
                ))}
              </select>
            </div>
          </div>

          {/* Properties */}
          <div style={{ borderTop: "1px solid #F3F4F6", paddingTop: 6 }}>
            {styleClass.properties.length === 0 && !showPropSelector && (
              <div style={{ fontSize: 12, color: "#9CA3AF", textAlign: "center", padding: 8 }}>
                暂无属性
              </div>
            )}
            {styleClass.properties.map((p) => (
              <PropertyRow key={p.id} property={p} classId={styleClass.id} componentName={componentName} variables={variables} />
            ))}

            {/* Add property UI */}
            {showPropSelector ? (
              <div style={{ marginTop: 4 }}>
                <input
                  type="text" value={propSearch} onChange={(e) => setPropSearch(e.target.value)}
                  placeholder="搜索 CSS 属性..."
                  aria-label="搜索 CSS 属性"
                  style={{
                    width: "100%", padding: "3px 6px", fontSize: 12, border: "1px solid #6366F1", borderRadius: 3,
                    outline: "2px solid transparent", boxSizing: "border-box",
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") { setShowPropSelector(false); setPropSearch(""); }
                    if (e.key === "Enter" && filteredProps.length > 0) { addProperty(filteredProps[0]); }
                  }}
                  spellCheck={false}
                />
                <div style={{ maxHeight: 120, overflow: "auto", marginTop: 2, border: "1px solid #E5E7EB", borderRadius: 3 }}>
                  {filteredProps.slice(0, 30).map((p) => (
                    <div
                      key={p}
                      role="button"
                      tabIndex={0}
                      onMouseDown={(e) => { e.preventDefault(); addProperty(p); }}
                      style={{
                        padding: "3px 6px", fontSize: 12, fontFamily: "monospace",
                        cursor: "pointer", color: "#374151",
                        background: "transparent",
                        borderBottom: "1px solid #F3F4F6",
                      }}
                      onMouseEnter={(e) => { (e.target as HTMLElement).style.background = "#EEF2FF"; }}
                      onMouseLeave={(e) => { (e.target as HTMLElement).style.background = "transparent"; }}
                    >
                      {p}
                      {DROPDOWN_PROPERTIES[p] !== undefined && <span style={{ color: "#9CA3AF", marginLeft: 4, fontSize: 12 }}>下拉</span>}
                      {PROPERTY_UNIT_MAP[p] !== undefined && <span style={{ color: "#9CA3AF", marginLeft: 4, fontSize: 12 }}>单位</span>}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <button type="button"
                onClick={() => setShowPropSelector(true)}
                style={{
                  width: "100%", padding: "3px", fontSize: 12, fontWeight: 500,
                  border: "1px dashed #D1D5DB", borderRadius: 3,
                  background: "transparent", color: "#6B7280", cursor: "pointer",
                  marginTop: 4,
                }}
              >
                + 新建属性
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
// PropertyRow
// ═══════════════════════════════════════════════════════════════════

function PropertyRow({ property, classId, componentName, variables }: {
  property: StyleProperty;
  classId: string;
  componentName: string;
  variables: CssVariable[];
}) {
  const [propName, setPropName] = useState(property.name);
  useEffect(() => { setPropName(property.name); }, [property.name]);
  const [val, setVal] = useState(
    property.segments.length > 0 && property.segments[0].type === "text"
      ? property.segments[0].value
      : ""
  );
  useEffect(() => {
    setVal(
      property.segments.length > 0 && property.segments[0].type === "text"
        ? property.segments[0].value
        : ""
    );
  }, [property.segments]);

  const dropdownOpts = DROPDOWN_PROPERTIES[propName];
  const unitGroup = PROPERTY_UNIT_MAP[propName];

  const commitName = () => {
    usePoiesisStore.getState().updateProperty(componentName, classId, property.id, { name: propName });
  };

  const commitValue = () => {
    usePoiesisStore.getState().updateProperty(componentName, classId, property.id, {
      segments: val.trim() ? [{ type: "text", value: val.trim() }] : [],
    });
  };

  return (
    <div style={{
      display: "flex", alignItems: "flex-start", gap: 4, padding: "2px 0",
      borderBottom: "1px solid #F9FAFB",
    }}>
      {/* Property name */}
      <div style={{ width: 100, flexShrink: 0 }}>
        {/* Allow editing prop name via click */}
        <select
          aria-label="Property name"
          value={propName}
          onChange={(e) => { setPropName(e.target.value); commitName(); }}
          onBlur={commitName}
          style={{
            width: "100%", padding: "2px 2px", fontSize: 12, fontFamily: "monospace",
            border: "1px solid #E5E7EB", borderRadius: 3, color: "#374151",
            background: "#fff",
          }}
        >
          <option value={propName}>{propName}</option>
          {ALL_CSS_PROPS.filter((p) => p !== propName).map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>
      </div>
      {/* Value */}
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* If dropdown property, use select */}
        {Array.isArray(dropdownOpts) ? (
          <select
            aria-label="Property value"
            value={val}
            onChange={(e) => { setVal(e.target.value); }}
            onBlur={commitValue}
            style={{
              width: "100%", padding: "2px 4px", fontSize: 12, fontFamily: "monospace",
              border: "1px solid #E5E7EB", borderRadius: 3,
              color: "#374151", background: "#fff",
            }}
          >
            <option value="">--</option>
            {dropdownOpts.map((o) => (
              <option key={o} value={o}>{o}</option>
            ))}
          </select>
        ) : (
          <FormulaValueInput
            value={val}
            onChange={(v) => { setVal(v); }}
            onBlur={commitValue}
            variables={variables}
            placeholder={unitGroup ? `数值 (${unitGroup.default})` : "CSS 值"}
          />
        )}
      </div>
      <button type="button" style={S.deleteBtn} onClick={() => usePoiesisStore.getState().removeProperty(componentName, classId, property.id)}>
        ×
      </button>
    </div>
  );
}
