// components/design/ClassEditor.tsx
// Full class editor with grouped properties, pseudo-class tabs, and token binding.

import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils";
import type { StyleProperty, StyleValueSegment } from "../../core/types/style";
import type { DesignClass, PseudoState } from "../../core/types/design";

// ── Constants ──

const PSEUDO_TABS = ["base", "hover", "focus", "active", "disabled", "focus-visible"] as const;

const ADDITIONAL_PSEUDOS = [
  "focus-within", "visited", "placeholder", "first-child",
  "last-child", "nth-child", "checked", "required", "invalid", "read-only",
];

interface GroupDef {
  label: string;
  properties: string[];
}

const PROPERTY_GROUPS: GroupDef[] = [
  {
    label: "布局 Layout",
    properties: [
      "display", "position", "width", "height",
      "flexDirection", "justifyContent", "alignItems", "alignSelf",
      "padding", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
      "margin", "marginTop", "marginRight", "marginBottom", "marginLeft",
      "gap", "overflow", "overflowX", "overflowY", "flexWrap",
    ],
  },
  {
    label: "视觉 Visual",
    properties: [
      "background", "border", "borderColor", "borderRadius",
      "boxShadow", "opacity", "cursor", "transition", "transform",
    ],
  },
  {
    label: "文字 Typography",
    properties: [
      "color", "fontFamily", "fontSize", "fontWeight",
      "lineHeight", "textAlign", "textDecoration", "textTransform", "letterSpacing",
    ],
  },
];

const ALL_PROPERTY_NAMES = PROPERTY_GROUPS.flatMap((g) => g.properties);

const PROPERTY_LABELS: Record<string, string> = {
  display: "Display",
  position: "定位",
  width: "宽度",
  height: "高度",
  flexDirection: "Flex 方向",
  justifyContent: "主轴对齐",
  alignItems: "交叉轴对齐",
  alignSelf: "自身对齐",
  padding: "内边距",
  paddingTop: "内边距-上",
  paddingRight: "内边距-右",
  paddingBottom: "内边距-下",
  paddingLeft: "内边距-左",
  margin: "外边距",
  marginTop: "外边距-上",
  marginRight: "外边距-右",
  marginBottom: "外边距-下",
  marginLeft: "外边距-左",
  gap: "间距",
  overflow: "溢出",
  overflowX: "溢出-X",
  overflowY: "溢出-Y",
  flexWrap: "换行",
  background: "背景色",
  border: "边框",
  borderColor: "边框颜色",
  borderRadius: "圆角",
  boxShadow: "阴影",
  opacity: "不透明度",
  cursor: "光标",
  transition: "过渡",
  transform: "变换",
  color: "文字颜色",
  fontFamily: "字体",
  fontSize: "字号",
  fontWeight: "字重",
  lineHeight: "行高",
  textAlign: "对齐",
  textDecoration: "装饰线",
  textTransform: "大小写",
  letterSpacing: "字间距",
};

const ENUM_OPTIONS: Record<string, string[]> = {
  display: ["block", "flex", "inline", "inline-block", "grid", "none", "contents"],
  position: ["static", "relative", "absolute", "fixed", "sticky"],
  flexDirection: ["row", "column", "row-reverse", "column-reverse"],
  justifyContent: ["flex-start", "center", "flex-end", "space-between", "space-around", "space-evenly"],
  alignItems: ["stretch", "flex-start", "center", "flex-end", "baseline"],
  alignSelf: ["auto", "stretch", "flex-start", "center", "flex-end", "baseline"],
  overflow: ["visible", "hidden", "scroll", "auto"],
  overflowX: ["visible", "hidden", "scroll", "auto"],
  overflowY: ["visible", "hidden", "scroll", "auto"],
  flexWrap: ["nowrap", "wrap", "wrap-reverse"],
  textAlign: ["left", "center", "right", "justify"],
  textDecoration: ["none", "underline", "line-through", "overline"],
  textTransform: ["none", "uppercase", "lowercase", "capitalize"],
  fontWeight: ["100", "200", "300", "400", "500", "600", "700", "800", "900"],
  cursor: ["auto", "default", "pointer", "text", "move", "not-allowed", "grab", "zoom-in", "zoom-out"],
};

// ── Helpers ──

function segmentsToString(segments: StyleValueSegment[]): string {
  return segments.map((s) => (s.type === "text" ? s.value : "")).join("");
}

function stringToSegments(value: string): StyleValueSegment[] {
  return value ? [{ type: "text" as const, value }] : [];
}

function hasTokenBinding(segments: StyleValueSegment[]): boolean {
  return segments.some((s) => s.type === "var-ref");
}

function getTokenRef(segments: StyleValueSegment[]): string | null {
  const ref = segments.find((s) => s.type === "var-ref");
  return ref?.name ?? null;
}

function makeVarRefSegments(name: string): StyleValueSegment[] {
  return [{ type: "var-ref" as const, name }];
}

function isEnumProperty(name: string): boolean {
  return name in ENUM_OPTIONS;
}

// ── Styles ──

const SIDEBAR_STYLE: React.CSSProperties = {
  width: 180,
  borderRight: "1px solid #e2e8f0",
  display: "flex",
  flexDirection: "column",
  flexShrink: 0,
};

const SIDEBAR_HEADER: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "10px 8px 6px",
};

const CLASS_ITEM_BASE: React.CSSProperties = {
  padding: "6px 12px",
  fontSize: 12,
  cursor: "pointer",
  color: "#374151",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  borderRadius: 4,
  margin: "1px 6px",
};

const CLASS_ITEM_ACTIVE: React.CSSProperties = {
  ...CLASS_ITEM_BASE,
  background: "#eef2ff",
  color: "#4f46e5",
  fontWeight: 600,
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "3px 6px",
  fontSize: 11,
  fontFamily: "monospace",
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  outline: "none",
  background: "#fff",
  color: "#374151",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "3px 4px",
  fontSize: 10,
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  outline: "none",
  background: "#fff",
  color: "#374151",
  cursor: "pointer",
  flex: 1,
  minWidth: 0,
};

const PROP_ROW: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "3px 0",
};

const PROP_LABEL: React.CSSProperties = {
  fontSize: 11,
  color: "#374151",
  width: 80,
  flexShrink: 0,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const INHERITED_LABEL: React.CSSProperties = {
  ...PROP_LABEL,
  color: "#94a3b8",
};

const GROUP_HEADER: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "10px 0 4px",
  marginBottom: 4,
};

const LINK_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  fontSize: 12,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
  color: "#94a3b8",
};

const LINK_BTN_ACTIVE: React.CSSProperties = {
  ...LINK_BTN,
  color: "#6366f1",
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "#94a3b8",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
};

// ── Hook: outside click ──

function useOutsideClick(
  ref: React.RefObject<HTMLElement | null>,
  enabled: boolean,
  onOutside: () => void,
) {
  useEffect(() => {
    if (!enabled) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onOutside();
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [enabled, ref, onOutside]);
}

// ── Component ──

export function ClassEditor() {
  // ── Store ──
  const classes = usePoiesisStore((s) => s.activeDesign.classes);
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const addDesignClass = usePoiesisStore((s) => s.addDesignClass);
  const updateDesignClass = usePoiesisStore((s) => s.updateDesignClass);
  const removeDesignClass = usePoiesisStore((s) => s.removeDesignClass);

  // ── Local state ──
  const [selectedClassId, setSelectedClassId] = useState<string | null>(null);
  const [activePseudo, setActivePseudo] = useState<string>("base");
  const [showPseudoMenu, setShowPseudoMenu] = useState(false);
  const [showAddProp, setShowAddProp] = useState(false);
  const [addPropSearch, setAddPropSearch] = useState("");
  const [tokenPickerProp, setTokenPickerProp] = useState<{
    classId: string;
    pseudo: string;
    propName: string;
  } | null>(null);
  const [editingClassName, setEditingClassName] = useState<string | null>(null);

  const pseudoMenuRef = useRef<HTMLDivElement>(null);
  const addPropRef = useRef<HTMLDivElement>(null);
  const tokenPickerRef = useRef<HTMLDivElement>(null);

  // ── Derived ──

  const selectedClass = useMemo(
    () => classes.find((c) => c.id === selectedClassId) ?? null,
    [classes, selectedClassId],
  );

  const pseudoState = useMemo(() => {
    if (!selectedClass || activePseudo === "base") return null;
    return selectedClass.pseudoStates.find((ps) => ps.pseudo === activePseudo) ?? null;
  }, [selectedClass, activePseudo]);

  // ── Close popups on outside click ──

  useOutsideClick(pseudoMenuRef, showPseudoMenu, () => setShowPseudoMenu(false));
  useOutsideClick(addPropRef, showAddProp, () => {
    setShowAddProp(false);
    setAddPropSearch("");
  });
  useOutsideClick(tokenPickerRef, !!tokenPickerProp, () => setTokenPickerProp(null));

  // ── Handlers ──

  /** Build a new properties array with one property added or updated. */
  const updatePropsList = useCallback(
    (props: StyleProperty[], propName: string, value: string): StyleProperty[] => {
      const idx = props.findIndex((p) => p.name === propName);
      if (idx >= 0) {
        const next = [...props];
        next[idx] = { ...next[idx], segments: stringToSegments(value) };
        return next;
      }
      return [...props, { id: uid("prop"), name: propName, segments: stringToSegments(value) }];
    },
    [],
  );

  const setPropToken = useCallback(
    (props: StyleProperty[], propName: string, tokenName: string): StyleProperty[] => {
      const idx = props.findIndex((p) => p.name === propName);
      if (idx >= 0) {
        const next = [...props];
        next[idx] = { ...next[idx], segments: makeVarRefSegments(tokenName) };
        return next;
      }
      return [...props, { id: uid("prop"), name: propName, segments: makeVarRefSegments(tokenName) }];
    },
    [],
  );

  const removePropFromList = useCallback(
    (props: StyleProperty[], propName: string): StyleProperty[] => {
      return props.filter((p) => p.name !== propName);
    },
    [],
  );

  /** Ensure a pseudo-state exists for the active pseudo, or create one. */
  const ensurePseudoState = useCallback(
    (cls: DesignClass): { pseudoStates: PseudoState[]; psIdx: number } => {
      const pseudoStates = [...cls.pseudoStates];
      const existing = pseudoStates.findIndex((ps) => ps.pseudo === activePseudo);
      if (existing >= 0) return { pseudoStates, psIdx: existing };
      const newPs: PseudoState = {
        id: uid("ps"),
        pseudo: activePseudo,
        properties: [],
      };
      pseudoStates.push(newPs);
      return { pseudoStates, psIdx: pseudoStates.length - 1 };
    },
    [activePseudo],
  );

  const handleSelectClass = useCallback((id: string) => {
    setSelectedClassId(id);
    setActivePseudo("base");
  }, []);

  const handleAddClass = useCallback(() => {
    addDesignClass(`class-${classes.length + 1}`);
  }, [classes.length, addDesignClass]);

  const handleRemoveClass = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.stopPropagation();
      removeDesignClass(id);
      if (selectedClassId === id) setSelectedClassId(null);
    },
    [removeDesignClass, selectedClassId],
  );

  const handleClassNameDone = useCallback(
    (id: string, name: string) => {
      if (name.trim()) updateDesignClass(id, { name: name.trim() });
      setEditingClassName(null);
    },
    [updateDesignClass],
  );

  const handlePseudoSelect = useCallback((pseudo: string) => {
    setActivePseudo(pseudo);
    setShowPseudoMenu(false);
  }, []);

  /** Update a property value in base or pseudo state. */
  const handlePropChange = useCallback(
    (propName: string, value: string) => {
      if (!selectedClass) return;

      if (activePseudo === "base") {
        updateDesignClass(selectedClass.id, {
          properties: updatePropsList(selectedClass.properties, propName, value),
        });
      } else {
        const { pseudoStates, psIdx } = ensurePseudoState(selectedClass);
        pseudoStates[psIdx] = {
          ...pseudoStates[psIdx],
          properties: updatePropsList(pseudoStates[psIdx].properties, propName, value),
        };
        updateDesignClass(selectedClass.id, { pseudoStates });
      }
    },
    [selectedClass, activePseudo, updateDesignClass, updatePropsList, ensurePseudoState],
  );

  /** Remove a property from base or pseudo state. */
  const handleRemoveProp = useCallback(
    (propName: string) => {
      if (!selectedClass) return;

      if (activePseudo === "base") {
        updateDesignClass(selectedClass.id, {
          properties: removePropFromList(selectedClass.properties, propName),
        });
      } else {
        const pseudoStates = selectedClass.pseudoStates.map((ps) => {
          if (ps.pseudo !== activePseudo) return ps;
          return { ...ps, properties: removePropFromList(ps.properties, propName) };
        });
        updateDesignClass(selectedClass.id, { pseudoStates });
      }
    },
    [selectedClass, activePseudo, updateDesignClass, removePropFromList],
  );

  /** Toggle token picker for a property. */
  const handleToggleTokenLink = useCallback(
    (propName: string) => {
      if (
        tokenPickerProp?.propName === propName &&
        tokenPickerProp?.pseudo === activePseudo
      ) {
        setTokenPickerProp(null);
      } else {
        setTokenPickerProp({
          classId: selectedClass?.id ?? "",
          pseudo: activePseudo,
          propName,
        });
      }
    },
    [tokenPickerProp, activePseudo, selectedClass],
  );

  /** Bind a token to a property via var-ref segment. */
  const handleSelectToken = useCallback(
    (tokenName: string) => {
      if (!selectedClass || !tokenPickerProp) return;
      const propName = tokenPickerProp.propName;

      if (activePseudo === "base") {
        updateDesignClass(selectedClass.id, {
          properties: setPropToken(selectedClass.properties, propName, tokenName),
        });
      } else {
        const { pseudoStates, psIdx } = ensurePseudoState(selectedClass);
        pseudoStates[psIdx] = {
          ...pseudoStates[psIdx],
          properties: setPropToken(pseudoStates[psIdx].properties, propName, tokenName),
        };
        updateDesignClass(selectedClass.id, { pseudoStates });
      }
      setTokenPickerProp(null);
    },
    [selectedClass, tokenPickerProp, activePseudo, updateDesignClass, setPropToken, ensurePseudoState],
  );

  /** Remove token binding from a property (falls back to empty text). */
  const handleUnlinkToken = useCallback(() => {
    if (!selectedClass || !tokenPickerProp) return;
    const propName = tokenPickerProp.propName;

    if (activePseudo === "base") {
      updateDesignClass(selectedClass.id, {
        properties: removePropFromList(selectedClass.properties, propName),
      });
    } else {
      const pseudoStates = selectedClass.pseudoStates.map((ps) => {
        if (ps.pseudo !== activePseudo) return ps;
        return { ...ps, properties: removePropFromList(ps.properties, propName) };
      });
      updateDesignClass(selectedClass.id, { pseudoStates });
    }
    setTokenPickerProp(null);
  }, [selectedClass, tokenPickerProp, activePseudo, updateDesignClass, removePropFromList]);

  /** Add a new (empty) property. */
  const handleAddProperty = useCallback(
    (propName: string) => {
      if (!selectedClass) return;
      handlePropChange(propName, "");
      setShowAddProp(false);
      setAddPropSearch("");
    },
    [selectedClass, handlePropChange],
  );

  // ── Render helpers ──

  const renderSidebar = () => (
    <div style={SIDEBAR_STYLE}>
      <div style={SIDEBAR_HEADER}>Classes ({classes.length})</div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {classes.map((cls) => (
          <div
            key={cls.id}
            onClick={() => handleSelectClass(cls.id)}
            style={cls.id === selectedClassId ? CLASS_ITEM_ACTIVE : CLASS_ITEM_BASE}
          >
            <span
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                flex: 1,
              }}
              onDoubleClick={() => setEditingClassName(cls.id)}
            >
              {cls.name}
            </span>
            <button
              onClick={(e) => handleRemoveClass(e, cls.id)}
              style={REMOVE_BTN}
              title="Remove class"
            >
              &times;
            </button>
          </div>
        ))}
      </div>
      <button
        onClick={handleAddClass}
        style={{
          border: "1px dashed #cbd5e1",
          borderRadius: 4,
          background: "transparent",
          color: "#64748b",
          fontSize: 11,
          padding: "4px 10px",
          cursor: "pointer",
          margin: "8px",
        }}
      >
        + Add Class
      </button>
    </div>
  );

  const renderPseudoTabs = () => {
    const pseudoExists = pseudoState !== null || activePseudo === "base";

    return (
      <div
        style={{
          display: "flex",
          gap: 0,
          borderBottom: "1px solid #e2e8f0",
          padding: "0 4px",
          alignItems: "center",
          flexShrink: 0,
        }}
      >
        {PSEUDO_TABS.map((pseudo) => (
          <button
            key={pseudo}
            onClick={() => handlePseudoSelect(pseudo)}
            style={{
              padding: "6px 10px",
              fontSize: 11,
              cursor: "pointer",
              border: "none",
              background: "none",
              color: activePseudo === pseudo ? "#4f46e5" : "#64748b",
              fontWeight: activePseudo === pseudo ? 600 : 400,
              borderBottom:
                activePseudo === pseudo ? "2px solid #6366f1" : "2px solid transparent",
            }}
          >
            {pseudo === "base" ? "Base" : `:${pseudo}`}
          </button>
        ))}
        <div style={{ position: "relative" }}>
          <button
            onClick={() => setShowPseudoMenu((v) => !v)}
            style={{
              padding: "6px 6px",
              fontSize: 16,
              cursor: "pointer",
              border: "none",
              background: "none",
              color: "#64748b",
              lineHeight: 1,
            }}
            title="More pseudo-classes"
          >
            +
          </button>
          {showPseudoMenu && (
            <div
              ref={pseudoMenuRef}
              style={{
                position: "absolute",
                top: "100%",
                left: 0,
                marginTop: 2,
                background: "#fff",
                border: "1px solid #e2e8f0",
                borderRadius: 6,
                boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
                zIndex: 100,
                minWidth: 140,
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {ADDITIONAL_PSEUDOS.map((pseudo) => (
                <button
                  key={pseudo}
                  onClick={() => handlePseudoSelect(pseudo)}
                  style={{
                    display: "block",
                    width: "100%",
                    padding: "6px 12px",
                    fontSize: 11,
                    border: "none",
                    background: "transparent",
                    color: "#374151",
                    cursor: "pointer",
                    textAlign: "left",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "transparent";
                  }}
                >
                  :{pseudo}
                </button>
              ))}
            </div>
          )}
        </div>
        <div style={{ flex: 1 }} />
        {!pseudoExists && activePseudo !== "base" && (
          <span style={{ fontSize: 10, color: "#94a3b8", paddingRight: 8 }}>
            (no overrides yet)
          </span>
        )}
      </div>
    );
  };

  const renderPropertyRow = (
    propName: string,
    prop: StyleProperty | undefined,
    inherited: boolean,
  ) => {
    if (!selectedClass) return null;
    const isEnum = isEnumProperty(propName);
    const label = PROPERTY_LABELS[propName] || propName;
    const displayValue = prop ? segmentsToString(prop.segments) : "";
    const hasLink = prop ? hasTokenBinding(prop.segments) : false;

    return (
      <div
        key={propName}
        style={PROP_ROW}
        title={inherited && activePseudo !== "base" ? "继承自 Base" : undefined}
      >
        {/* Override dot */}
        {!inherited && activePseudo !== "base" && (
          <div
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: "#6366f1",
              flexShrink: 0,
            }}
          />
        )}
        {inherited && <div style={{ width: 6, flexShrink: 0 }} />}

        {/* Label */}
        <span style={inherited ? INHERITED_LABEL : PROP_LABEL} title={label}>
          {label}
        </span>

        {/* Value */}
        {!inherited && isEnum && (
          <select
            value={displayValue}
            onChange={(e) => handlePropChange(propName, e.target.value)}
            style={SELECT_STYLE}
          >
            <option value="">—</option>
            {ENUM_OPTIONS[propName].map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        )}
        {!inherited && !isEnum && (
          <input
            value={displayValue}
            onChange={(e) => handlePropChange(propName, e.target.value)}
            style={INPUT_STYLE}
            placeholder="value"
          />
        )}
        {inherited && (
          <div
            style={{
              ...INPUT_STYLE,
              background: "#f8fafc",
              color: "#94a3b8",
              cursor: "not-allowed",
              display: "flex",
              alignItems: "center",
            }}
          >
            {displayValue || "—"}
          </div>
        )}

        {/* Token link */}
        {!inherited && (
          <button
            onClick={() => handleToggleTokenLink(propName)}
            style={hasLink ? LINK_BTN_ACTIVE : LINK_BTN}
            title={hasLink ? `Token: ${getTokenRef(prop!.segments)}` : "Link token"}
          >
            &#128279;
          </button>
        )}

        {/* Remove */}
        {!inherited && (
          <button onClick={() => handleRemoveProp(propName)} style={REMOVE_BTN} title="Remove property">
            &times;
          </button>
        )}
      </div>
    );
  };

  const renderTokenPicker = () => {
    if (!tokenPickerProp) return null;

    return (
      <div
        ref={tokenPickerRef}
        style={{
          background: "#fff",
          border: "1px solid #e2e8f0",
          borderRadius: 8,
          boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
          zIndex: 1000,
          minWidth: 200,
          maxHeight: 260,
          overflow: "auto",
          padding: 4,
        }}
      >
        <div
          style={{
            fontSize: 10,
            fontWeight: 600,
            color: "#6b7280",
            padding: "4px 8px",
            textTransform: "uppercase",
            letterSpacing: "0.5px",
          }}
        >
          Link Token
        </div>
        {tokens.length === 0 && (
          <div
            style={{
              fontSize: 10,
              color: "#94a3b8",
              padding: 8,
              textAlign: "center",
            }}
          >
            No tokens defined
          </div>
        )}
        {["colors", "spacing", "typography", "radii"].map((cat) => {
          const catTokens = tokens.filter((t) => t.category === cat);
          if (catTokens.length === 0) return null;
          return (
            <div key={cat}>
              <div
                style={{
                  fontSize: 9,
                  fontWeight: 600,
                  color: "#94a3b8",
                  padding: "4px 8px 2px",
                  textTransform: "uppercase",
                }}
              >
                {cat}
              </div>
              {catTokens.map((t) => (
                <button
                  key={t.id}
                  onClick={() => handleSelectToken(t.name)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    width: "100%",
                    padding: "4px 8px",
                    fontSize: 11,
                    border: "none",
                    background: "transparent",
                    color: "#374151",
                    cursor: "pointer",
                    textAlign: "left",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "transparent";
                  }}
                >
                  {t.tokenType === "color" && (
                    <span
                      style={{
                        width: 12,
                        height: 12,
                        borderRadius: 2,
                        background: t.value,
                        border: "1px solid #cbd5e1",
                        flexShrink: 0,
                      }}
                    />
                  )}
                  <span>{t.name}</span>
                  <span
                    style={{
                      marginLeft: "auto",
                      fontSize: 9,
                      color: "#94a3b8",
                      fontFamily: "monospace",
                    }}
                  >
                    {t.value}
                  </span>
                </button>
              ))}
            </div>
          );
        })}
        <div style={{ borderTop: "1px solid #e2e8f0", marginTop: 4, paddingTop: 4 }}>
          <button
            onClick={handleUnlinkToken}
            style={{
              display: "block",
              width: "100%",
              padding: "4px 8px",
              fontSize: 11,
              border: "none",
              background: "transparent",
              color: "#ef4444",
              cursor: "pointer",
              textAlign: "left",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "#fef2f2";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
            }}
          >
            Unlink token
          </button>
        </div>
      </div>
    );
  };

  const renderAddProperty = () => {
    const filtered = ALL_PROPERTY_NAMES.filter(
      (n) => n.toLowerCase().includes(addPropSearch.toLowerCase()),
    );

    return (
      <div style={{ position: "relative", marginTop: 4 }}>
        <button
          onClick={() => setShowAddProp((v) => !v)}
          style={{
            border: "1px dashed #cbd5e1",
            borderRadius: 4,
            background: "transparent",
            color: "#64748b",
            fontSize: 10,
            padding: "4px 10px",
            cursor: "pointer",
            width: "100%",
          }}
        >
          + Add property
        </button>
        {showAddProp && (
          <div
            ref={addPropRef}
            style={{
              position: "absolute",
              top: "100%",
              left: 0,
              right: 0,
              marginTop: 2,
              background: "#fff",
              border: "1px solid #e2e8f0",
              borderRadius: 6,
              boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
              zIndex: 100,
              maxHeight: 200,
              overflow: "auto",
            }}
          >
            <input
              autoFocus
              value={addPropSearch}
              onChange={(e) => setAddPropSearch(e.target.value)}
              placeholder="Search properties..."
              style={{
                ...INPUT_STYLE,
                margin: 4,
                width: "calc(100% - 8px)",
                padding: "4px 6px",
                fontSize: 11,
              }}
            />
            {filtered.map((name) => (
              <button
                key={name}
                onClick={() => handleAddProperty(name)}
                style={{
                  display: "block",
                  width: "100%",
                  padding: "4px 10px",
                  fontSize: 11,
                  border: "none",
                  background: "transparent",
                  color: "#374151",
                  cursor: "pointer",
                  textAlign: "left",
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "transparent";
                }}
              >
                <span style={{ color: "#64748b", marginRight: 4 }}>+</span>
                {PROPERTY_LABELS[name] || name}
              </button>
            ))}
            {filtered.length === 0 && (
              <div
                style={{
                  padding: 8,
                  fontSize: 10,
                  color: "#94a3b8",
                  textAlign: "center",
                }}
              >
                No matching properties
              </div>
            )}
          </div>
        )}
      </div>
    );
  };

  const renderPropertyGroups = () => {
    if (!selectedClass) return null;

    if (activePseudo === "base") {
      // Base tab: show only properties that have been explicitly set
      const currentProps = selectedClass.properties;

      return (
        <>
          {PROPERTY_GROUPS.map((group) => {
            const groupProps = group.properties.filter((gp) =>
              currentProps.some((cp) => cp.name === gp),
            );
            if (groupProps.length === 0) return null;
            return (
              <div key={group.label} style={{ marginBottom: 8 }}>
                <div style={GROUP_HEADER}>{group.label}</div>
                {groupProps.map((name) => {
                  const prop = currentProps.find((p) => p.name === name);
                  return renderPropertyRow(name, prop, false);
                })}
              </div>
            );
          })}
          {renderAddProperty()}
        </>
      );
    }

    // Pseudo tab: show inherited + overridden properties
    const baseProps = selectedClass.properties;
    const pseudoProps = pseudoState?.properties ?? [];

    // Collect property names from base and pseudo
    const displayedNames = new Set<string>();
    for (const bp of baseProps) displayedNames.add(bp.name);
    for (const pp of pseudoProps) displayedNames.add(pp.name);

    return (
      <>
        {PROPERTY_GROUPS.map((group) => {
          const groupNames = group.properties.filter((n) => displayedNames.has(n));
          if (groupNames.length === 0) return null;
          return (
            <div key={group.label} style={{ marginBottom: 8 }}>
              <div style={GROUP_HEADER}>{group.label}</div>
              {groupNames.map((name) => {
                const baseProp = baseProps.find((p) => p.name === name);
                const pseudoProp = pseudoProps.find((p) => p.name === name);
                const baseVal = baseProp ? segmentsToString(baseProp.segments) : "";
                const pseudoVal = pseudoProp ? segmentsToString(pseudoProp.segments) : "";
                const isOverridden = pseudoProp !== undefined && pseudoVal !== baseVal;
                const isNew = pseudoProp !== undefined && !baseProp;
                const editable = isOverridden || isNew;
                return renderPropertyRow(
                  name,
                  editable ? pseudoProp : baseProp,
                  !editable,
                );
              })}
            </div>
          );
        })}
        {renderAddProperty()}
      </>
    );
  };

  // ── Main render ──

  return (
    <div
      style={{
        display: "flex",
        height: "100%",
        fontSize: 12,
        color: "#374151",
        position: "relative",
      }}
    >
      {renderSidebar()}

      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          position: "relative",
        }}
      >
        {selectedClass ? (
          <>
            {/* Class name header */}
            <div
              style={{
                padding: "8px 12px",
                fontSize: 13,
                fontWeight: 600,
                color: "#1e293b",
                borderBottom: "1px solid #e2e8f0",
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              {editingClassName === selectedClass.id ? (
                <input
                  autoFocus
                  defaultValue={selectedClass.name}
                  onBlur={(e) => handleClassNameDone(selectedClass.id, e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter")
                      handleClassNameDone(
                        selectedClass.id,
                        (e.target as HTMLInputElement).value,
                      );
                    if (e.key === "Escape") setEditingClassName(null);
                  }}
                  style={{
                    ...INPUT_STYLE,
                    fontSize: 13,
                    fontWeight: 600,
                    padding: "2px 6px",
                  }}
                />
              ) : (
                <span>{selectedClass.name}</span>
              )}
              <button
                onClick={() => setEditingClassName(selectedClass.id)}
                style={{
                  border: "none",
                  background: "none",
                  cursor: "pointer",
                  color: "#94a3b8",
                  fontSize: 11,
                  padding: 0,
                }}
                title="Rename"
              >
                &#9998;
              </button>
            </div>

            {/* Pseudo tabs */}
            {renderPseudoTabs()}

            {/* Property editor */}
            <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
              {renderPropertyGroups()}
            </div>
          </>
        ) : (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              height: "100%",
              color: "#94a3b8",
              fontSize: 12,
            }}
          >
            Select a class to edit
          </div>
        )}

        {/* Token picker popover */}
        {tokenPickerProp && (
          <div
            style={{
              position: "absolute",
              top: 100,
              right: 20,
              zIndex: 999,
            }}
          >
            {renderTokenPicker()}
          </div>
        )}
      </div>
    </div>
  );
}
