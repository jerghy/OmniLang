// components/design/ClassEditor.tsx
// Full class editor with grouped properties, pseudo-class tabs, and token binding.

import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils/id";
import type { StyleProperty, StyleValueSegment } from "../../core/types/style";
import type { DesignClass, PseudoState } from "../../core/types/design";
import { ClassPreview } from "./ClassPreview";
import { SizeCard } from "./cards/SizeCard";
import { BackgroundCard } from "./cards/BackgroundCard";
import { BorderCard } from "./cards/BorderCard";
import { RadiusCard } from "./cards/RadiusCard";
import { SpacingCard } from "./cards/SpacingCard";
import { FocusOverlay } from "./FocusOverlay";
import { CollapsibleGroup } from "./CollapsibleGroup";
import { IconButtonGroup } from "./IconButtonGroup";
import { ScrubInput } from "./ScrubInput";
import { PropertyRow } from "./PropertyRow";

// ── Constants ──

const PSEUDO_TABS = ["base", "hover", "focus", "active", "disabled", "focus-visible"] as const;
const LAYOUT_PROP_NAMES = ["display", "position", "flexDirection", "justifyContent", "alignItems", "alignSelf", "gap", "overflow", "flexWrap"];
const TYPOGRAPHY_PROP_NAMES = ["color", "fontFamily", "fontSize", "fontWeight", "lineHeight", "textAlign", "textDecoration", "textTransform", "letterSpacing"];

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

function makeVarRefSegments(name: string): StyleValueSegment[] {
  return [{ type: "var-ref" as const, name }];
}

function isEnumProperty(name: string): boolean {
  return name in ENUM_OPTIONS;
}

function getPropValue(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
  if (!prop || prop.segments.length === 0) return "";
  const seg = prop.segments[0];
  return seg.type === "text" ? seg.value : "";
}

function setPropValue(
  props: StyleProperty[],
  name: string,
  value: string,
): StyleProperty[] {
  const existing = props.find((p) => p.name === name);
  if (existing) {
    return props.map((p) =>
      p.name === name
        ? { ...p, segments: [{ type: "text" as const, value }] }
        : p,
    );
  }
  return [
    ...props,
    { id: uid("prop"), name, segments: [{ type: "text", value }] },
  ];
}

// ── Styles ──

const SIDEBAR_STYLE: React.CSSProperties = {
  width: 180,
  borderRight: "1px solid var(--dp-border, #e8e5f2)",
  display: "flex",
  flexDirection: "column",
  flexShrink: 0,
  background: "var(--dp-surface, #fff)",
};

const SIDEBAR_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, #6b6893)",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "10px 10px 6px",
};

const CLASS_ITEM_BASE: React.CSSProperties = {
  padding: "6px 10px",
  fontSize: 12,
  cursor: "pointer",
  color: "var(--dp-text-secondary, #6b6893)",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  borderRadius: "var(--dp-radius-sm, 6px)",
  margin: "1px 6px",
  transition: "background 100ms ease, color 100ms ease",
};

const CLASS_ITEM_ACTIVE: React.CSSProperties = {
  ...CLASS_ITEM_BASE,
  background: "var(--dp-accent-subtle, #eef0ff)",
  color: "var(--dp-accent, #6366f1)",
  fontWeight: 600,
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "3px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "3px 4px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
  cursor: "pointer",
  flex: 1,
  minWidth: 0,
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, #a09db8)",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
  borderRadius: 3,
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
  const [focusMode, setFocusMode] = useState<string | null>(null);

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

  const currentProps = useMemo(() => {
    if (!selectedClass) return [];
    if (activePseudo === "base") return selectedClass.properties;
    return pseudoState?.properties ?? [];
  }, [selectedClass, activePseudo, pseudoState]);

  const layoutSummary = useMemo(() => {
    const parts = currentProps.filter(
      (p) => LAYOUT_PROP_NAMES.includes(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  const typographySummary = useMemo(() => {
    const parts = currentProps.filter(
      (p) => TYPOGRAPHY_PROP_NAMES.includes(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  // Interactive preview dimensions for focus mode
  const previewWidth = useMemo(() => parseFloat(getPropValue(currentProps, "width")) || 0, [currentProps]);
  const previewHeight = useMemo(() => parseFloat(getPropValue(currentProps, "height")) || 0, [currentProps]);

  // ── Close popups on outside click ──

  const handleOutsidePseudo = useCallback(() => setShowPseudoMenu(false), []);
  const handleOutsideAddProp = useCallback(() => {
    setShowAddProp(false);
    setAddPropSearch("");
  }, []);
  const handleOutsideTokenPicker = useCallback(() => setTokenPickerProp(null), []);

  useOutsideClick(pseudoMenuRef, showPseudoMenu, handleOutsidePseudo);
  useOutsideClick(addPropRef, showAddProp, handleOutsideAddProp);
  useOutsideClick(tokenPickerRef, !!tokenPickerProp, handleOutsideTokenPicker);

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

  // ── Stabilized callbacks via refs (so React.memo children don't re-render on every state change) ──

  const handlePropsChangeRef = useRef<(newProps: StyleProperty[]) => void>(null!);
  useEffect(() => {
    handlePropsChangeRef.current = (newProps: StyleProperty[]) => {
      if (!selectedClass) return;
      if (activePseudo === "base") {
        updateDesignClass(selectedClass.id, { properties: newProps });
      } else {
        const { pseudoStates, psIdx } = ensurePseudoState(selectedClass);
        pseudoStates[psIdx] = { ...pseudoStates[psIdx], properties: newProps };
        updateDesignClass(selectedClass.id, { pseudoStates });
      }
    };
  });

  const handlePropsChange = useCallback(
    (newProps: StyleProperty[]) => {
      handlePropsChangeRef.current?.(newProps);
    },
    [],
  );

  const handleSinglePropChangeRef = useRef<(name: string, value: string) => void>(null!);
  useEffect(() => {
    handleSinglePropChangeRef.current = (name: string, value: string) => {
      if (!selectedClass) return;
      const props =
        activePseudo === "base"
          ? selectedClass.properties
          : pseudoState?.properties ?? [];
      handlePropsChangeRef.current?.(setPropValue(props, name, value));
    };
  });

  const handleSinglePropChange = useCallback(
    (name: string, value: string) => {
      handleSinglePropChangeRef.current?.(name, value);
    },
    [],
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
            role="button"
            tabIndex={0}
            onClick={() => handleSelectClass(cls.id)}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelectClass(cls.id); } }}
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
            <span
              onClick={(e) => handleRemoveClass(e, cls.id)}
              style={REMOVE_BTN}
              title="Remove class"
            >
              &times;
            </span>
          </div>
        ))}
      </div>
      <button type="button"
        onClick={handleAddClass}
        style={{
          border: "1px dashed var(--dp-border, #e8e5f2)",
          borderRadius: "var(--dp-radius-sm, 6px)",
          background: "transparent",
          color: "var(--dp-text-muted, #a09db8)",
          fontSize: 12,
          padding: "5px 10px",
          cursor: "pointer",
          margin: "8px",
          transition: "color 150ms ease, border-color 150ms ease",
          fontFamily: "inherit",
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--dp-accent, #6366f1)";
          (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-accent, #6366f1)";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--dp-text-muted, #a09db8)";
          (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-border, #e8e5f2)";
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
          borderBottom: "1px solid var(--dp-border, #e8e5f2)",
          padding: "4px 6px 0",
          alignItems: "center",
          flexShrink: 0,
          background: "var(--dp-bg, #f7f6fa)",
        }}
      >
        {PSEUDO_TABS.map((pseudo) => (
          <button type="button"
            key={pseudo}
            onClick={() => handlePseudoSelect(pseudo)}
            style={{
              padding: "5px 10px",
              fontSize: 12,
              cursor: "pointer",
              border: "none",
              background: "none",
              color: activePseudo === pseudo ? "var(--dp-accent, #6366f1)" : "var(--dp-text-muted, #a09db8)",
              fontWeight: activePseudo === pseudo ? 600 : 400,
              borderBottom:
                activePseudo === pseudo ? "2px solid var(--dp-accent, #6366f1)" : "2px solid transparent",
              transition: "color 150ms ease, border-color 150ms ease",
              fontFamily: "var(--dp-font-mono, monospace)",
            }}
          >
            {pseudo === "base" ? "Base" : `:${pseudo}`}
          </button>
        ))}
        <div style={{ position: "relative" }}>
          <button type="button"
            onClick={() => setShowPseudoMenu((v) => !v)}
            style={{
              padding: "5px 6px",
              fontSize: 14,
              cursor: "pointer",
              border: "none",
              background: "none",
              color: "var(--dp-text-muted, #a09db8)",
              lineHeight: 1,
              fontFamily: "inherit",
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
                <button type="button"
                  key={pseudo}
                  onClick={() => handlePseudoSelect(pseudo)}
                  style={{
                    display: "block",
                    width: "100%",
                    padding: "6px 12px",
                    fontSize: 12,
                    border: "none",
                    background: "transparent",
                    color: "var(--dp-text-secondary, #6b6893)",
                    cursor: "pointer",
                    textAlign: "left",
                    fontFamily: "var(--dp-font-mono, monospace)",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, #eef0ff)";
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
          <span style={{ fontSize: 12, color: "var(--dp-text-muted, #a09db8)", paddingRight: 8 }}>
            (no overrides yet)
          </span>
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
            fontSize: 12,
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
              fontSize: 12,
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
                  fontSize: 12,
                  fontWeight: 600,
                  color: "#94a3b8",
                  padding: "4px 8px 2px",
                  textTransform: "uppercase",
                }}
              >
                {cat}
              </div>
              {catTokens.map((t) => (
                <button type="button"
                  key={t.id}
                  onClick={() => handleSelectToken(t.name)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    width: "100%",
                    padding: "4px 8px",
                    fontSize: 12,
                    border: "none",
                    background: "transparent",
                    color: "var(--dp-text-secondary, #6b6893)",
                    cursor: "pointer",
                    textAlign: "left",
                    fontFamily: "var(--dp-font-mono, monospace)",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, #eef0ff)";
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
                      fontSize: 12,
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
          <button type="button"
            onClick={handleUnlinkToken}
            style={{
              display: "block",
              width: "100%",
              padding: "4px 8px",
              fontSize: 12,
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
      <div style={{ position: "relative", margin: "4px 8px" }}>
        <button type="button"
          onClick={() => setShowAddProp((v) => !v)}
          style={{
            border: "1px dashed var(--dp-border, #e8e5f2)",
            borderRadius: "var(--dp-radius-sm, 6px)",
            background: "transparent",
            color: "var(--dp-text-muted, #a09db8)",
            fontSize: 12,
            padding: "6px 10px",
            cursor: "pointer",
            width: "100%",
            fontFamily: "inherit",
            transition: "color 150ms ease, border-color 150ms ease",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.color = "var(--dp-accent, #6366f1)";
            (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-accent, #6366f1)";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.color = "var(--dp-text-muted, #a09db8)";
            (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-border, #e8e5f2)";
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
              aria-label="Search properties"
              value={addPropSearch}
              onChange={(e) => setAddPropSearch(e.target.value)}
              placeholder="Search properties..."
              style={{
                ...INPUT_STYLE,
                margin: 4,
                width: "calc(100% - 8px)",
                padding: "4px 6px",
                fontSize: 12,
              }}
            />
            {filtered.map((name) => (
              <button type="button"
                key={name}
                onClick={() => handleAddProperty(name)}
                style={{
                  display: "block",
                  width: "100%",
                  padding: "4px 10px",
                  fontSize: 12,
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
                  fontSize: 12,
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

  // renderPropertyGroups removed — replaced by card-based rendering above.


  const renderFocusCard = () => {
    if (!focusMode) return null;
    const sharedProps = {
      properties: currentProps,
      onChange: handlePropsChange,
    };
    switch (focusMode) {
      case "size":
        return (
          <div style={{ width: 260, padding: 8 }}>
            <SizeCard {...sharedProps} />
          </div>
        );
      case "background":
        return (
          <div style={{ width: 260, padding: 8 }}>
            <BackgroundCard {...sharedProps} tokens={tokens} />
          </div>
        );
      case "border":
        return (
          <div style={{ width: 260, padding: 8 }}>
            <BorderCard {...sharedProps} tokens={tokens} />
          </div>
        );
      case "radius":
        return (
          <div style={{ width: 260, padding: 8 }}>
            <RadiusCard {...sharedProps} />
          </div>
        );
      case "spacing":
        return (
          <div style={{ width: 260, padding: 8 }}>
            <SpacingCard {...sharedProps} />
          </div>
        );
      default:
        return null;
    }
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
                color: "var(--dp-text, #1e1b4b)",
                borderBottom: "1px solid var(--dp-border, #e8e5f2)",
                display: "flex",
                alignItems: "center",
                gap: 8,
                background: "var(--dp-surface, #fff)",
                flexShrink: 0,
              }}
            >
              {editingClassName === selectedClass.id ? (
                <input
                  aria-label="Class name"
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
              <button type="button"
                onClick={() => setEditingClassName(selectedClass.id)}
                style={{
                  border: "none",
                  background: "none",
                  cursor: "pointer",
                  color: "var(--dp-text-muted, #a09db8)",
                  fontSize: 12,
                  padding: 0,
                  borderRadius: 3,
                }}
                title="Rename"
              >
                &#9998;
              </button>
            </div>

            {/* Pseudo tabs */}
            {renderPseudoTabs()}

            {/* Property editor + Preview */}
            <div style={{ flex: 1, display: "flex", overflow: "hidden", minHeight: 0, position: "relative" }}>
              {focusMode === null ? (
                <>
                  {/* Property editor: cards + collapsed groups */}
                  <div style={{ flex: 1, overflow: "auto", padding: "8px 0", background: "var(--dp-bg, #f7f6fa)" }}>
                    {/* Cards for common properties */}
                    <SizeCard
                      properties={currentProps}
                      onChange={handlePropsChange}
                      onFocus={() => setFocusMode("size")}
                    />
                    <BackgroundCard
                      properties={currentProps}
                      tokens={tokens}
                      onChange={handlePropsChange}
                      onFocus={() => setFocusMode("background")}
                    />
                    <BorderCard
                      properties={currentProps}
                      tokens={tokens}
                      onChange={handlePropsChange}
                      onFocus={() => setFocusMode("border")}
                    />
                    <RadiusCard
                      properties={currentProps}
                      onChange={handlePropsChange}
                      onFocus={() => setFocusMode("radius")}
                    />
                    <SpacingCard
                      properties={currentProps}
                      onChange={handlePropsChange}
                      onFocus={() => setFocusMode("spacing")}
                    />

                    {/* Layout group for remaining layout properties */}
                    <CollapsibleGroup title="布局 Layout" summary={layoutSummary} defaultOpen={false}>
                      {LAYOUT_PROP_NAMES.map((name) => {
                        const value = getPropValue(currentProps, name);
                        const label = PROPERTY_LABELS[name] || name;
                        if (name === "flexDirection") {
                          return (
                            <PropertyRow key={name} label={label}>
                              <IconButtonGroup
                                options={[
                                  { value: "row", label: "→", title: "row" },
                                  { value: "column", label: "↓", title: "column" },
                                  { value: "row-reverse", label: "←", title: "row-reverse" },
                                  { value: "column-reverse", label: "↑", title: "column-reverse" },
                                ]}
                                value={(value || "row") as "row" | "column" | "row-reverse" | "column-reverse"}
                                onChange={(v) => handleSinglePropChange(name, v as string)}
                              />
                            </PropertyRow>
                          );
                        }
                        if (name === "gap") {
                          return (
                            <PropertyRow key={name} label={label}>
                              <ScrubInput
                                value={parseFloat(value) || 0}
                                unit="px"
                                min={0}
                                max={200}
                                onChange={(v) => handleSinglePropChange(name, `${v}px`)}
                              />
                            </PropertyRow>
                          );
                        }
                        if (isEnumProperty(name)) {
                          return (
                            <PropertyRow key={name} label={label}>
                              <select
                                value={value}
                                onChange={(e) => handleSinglePropChange(name, e.target.value)}
                                style={SELECT_STYLE}
                              >
                                <option value="">—</option>
                                {ENUM_OPTIONS[name]?.map((opt) => (
                                  <option key={opt} value={opt}>{opt}</option>
                                )) ?? null}
                              </select>
                            </PropertyRow>
                          );
                        }
                        return (
                          <PropertyRow key={name} label={label}>
                            <input
                              value={value}
                              onChange={(e) => handleSinglePropChange(name, e.target.value)}
                              style={INPUT_STYLE}
                              placeholder="value"
                              aria-label={label}
                            />
                          </PropertyRow>
                        );
                      })}
                    </CollapsibleGroup>

                    {/* Typography group */}
                    <CollapsibleGroup title="文字 Typography" summary={typographySummary} defaultOpen={false}>
                      {TYPOGRAPHY_PROP_NAMES.map((name) => {
                        const value = getPropValue(currentProps, name);
                        const label = PROPERTY_LABELS[name] || name;
                        if (["fontSize", "lineHeight", "letterSpacing"].includes(name)) {
                          const unit = name === "fontSize" ? "px" : (name === "letterSpacing" ? "px" : "");
                          return (
                            <PropertyRow key={name} label={label}>
                              <ScrubInput
                                value={parseFloat(value) || 0}
                                unit={unit}
                                min={0}
                                max={999}
                                onChange={(v) => handleSinglePropChange(name, unit ? `${v}${unit}` : `${v}`)}
                              />
                            </PropertyRow>
                          );
                        }
                        if (isEnumProperty(name)) {
                          return (
                            <PropertyRow key={name} label={label}>
                              <select
                                value={value}
                                onChange={(e) => handleSinglePropChange(name, e.target.value)}
                                style={SELECT_STYLE}
                              >
                                <option value="">—</option>
                                {ENUM_OPTIONS[name]?.map((opt) => (
                                  <option key={opt} value={opt}>{opt}</option>
                                )) ?? null}
                              </select>
                            </PropertyRow>
                          );
                        }
                        return (
                          <PropertyRow key={name} label={label}>
                            <input
                              value={value}
                              onChange={(e) => handleSinglePropChange(name, e.target.value)}
                              style={INPUT_STYLE}
                              placeholder="value"
                              aria-label={label}
                            />
                          </PropertyRow>
                        );
                      })}
                    </CollapsibleGroup>

                    {renderAddProperty()}
                  </div>

                  {/* Live preview */}
                  <ClassPreview
                    baseProperties={selectedClass.properties}
                    pseudoStates={selectedClass.pseudoStates}
                    className={selectedClass.name}
                    tokens={tokens}
                  />
                </>
              ) : (
                <FocusOverlay
                  visible={true}
                  onClose={() => setFocusMode(null)}
                  floatingCard={renderFocusCard()}
                >
                  <ClassPreview
                    baseProperties={selectedClass.properties}
                    pseudoStates={selectedClass.pseudoStates}
                    className={selectedClass.name}
                    tokens={tokens}
                    interactive
                    width={previewWidth}
                    height={previewHeight}
                    onResize={(w, h) => {
                      handleSinglePropChange("width", `${w}px`);
                      handleSinglePropChange("height", `${h}px`);
                    }}
                    onResizeEnd={(w, h) => {
                      handleSinglePropChange("width", `${w}px`);
                      handleSinglePropChange("height", `${h}px`);
                    }}
                  />
                </FocusOverlay>
              )}
            </div>
          </>
        ) : (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              height: "100%",
              color: "var(--dp-text-muted, #a09db8)",
              fontSize: 12,
              background: "var(--dp-bg, #f7f6fa)",
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
