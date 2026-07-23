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
import styles from "./ClassEditor.module.css";
import { CollapsibleGroup } from "./CollapsibleGroup";
import { IconButtonGroup } from "./IconButtonGroup";
import { ScrubInput } from "./ScrubInput";
import { PropertyRow } from "./PropertyRow";
import { ClassEditorSidebar } from "./ClassEditorSidebar";
import { TokenPickerPopover } from "./TokenPickerPopover";

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

const INPUT_STYLE: React.CSSProperties = {
  padding: "3px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, var(--ol-border))",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, var(--ol-surface))",
  color: "var(--dp-text, var(--ol-gray-700))",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "3px 4px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, var(--ol-border))",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, var(--ol-surface))",
  color: "var(--dp-text, var(--ol-gray-700))",
  cursor: "pointer",
  flex: 1,
  minWidth: 0,
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

  const handlePropsChangeRef = useRef<(newProps: StyleProperty[]) => void>(() => {});
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

  const handleSinglePropChangeRef = useRef<(name: string, value: string) => void>(() => {});
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

  const renderPseudoTabs = () => {
    const pseudoExists = pseudoState !== null || activePseudo === "base";

    return (
      <div
        style={{
          display: "flex",
          gap: 0,
          borderBottom: "1px solid var(--dp-border, var(--ol-border))",
          padding: "4px 6px 0",
          alignItems: "center",
          flexShrink: 0,
          background: "var(--dp-bg, var(--ol-gray-50))",
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
              color: activePseudo === pseudo ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-text-muted, var(--ol-text-muted))",
              fontWeight: activePseudo === pseudo ? 600 : 400,
              borderBottom:
                activePseudo === pseudo ? "2px solid var(--dp-accent, var(--ol-color-primary))" : "2px solid transparent",
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
            className={styles.pseudoMoreBtn}
            title="More pseudo-classes"
          >
            +
          </button>
          {showPseudoMenu && (
            <div
              ref={pseudoMenuRef}
              className={styles.pseudoMenuDropdown}
            >
              {ADDITIONAL_PSEUDOS.map((pseudo) => (
                <button type="button"
                  key={pseudo}
                  onClick={() => handlePseudoSelect(pseudo)}
                  className={styles.pseudoMenuItem}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, var(--ol-color-primary-subtle))";
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
          <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))", paddingRight: 8 }}>
            (no overrides yet)
          </span>
        )}
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
          className={styles.addPropBtn}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.color = "var(--dp-accent, var(--ol-color-primary))";
            (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-accent, var(--ol-color-primary))";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.color = "var(--dp-text-muted, var(--ol-text-muted))";
            (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-border, var(--ol-border))";
          }}
        >
          + Add property
        </button>
        {showAddProp && (
          <div
            ref={addPropRef}
            className={styles.addPropDropdown}
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
                className={styles.addPropItem}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "var(--ol-gray-100)";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "transparent";
                }}
              >
                <span style={{ color: "var(--ol-text-secondary)", marginRight: 4 }}>+</span>
                {PROPERTY_LABELS[name] || name}
              </button>
            ))}
            {filtered.length === 0 && (
              <div
                style={{
                  padding: 8,
                  fontSize: 12,
                  color: "var(--ol-text-muted)",
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
    <div className={styles.mainContainer}>
      <ClassEditorSidebar
        classes={classes}
        selectedClassId={selectedClassId}
        editingClassName={editingClassName}
        onSelectClass={handleSelectClass}
        onAddClass={handleAddClass}
        onRemoveClass={handleRemoveClass}
        onClassNameDone={handleClassNameDone}
        onEditingClassNameChange={setEditingClassName}
      />

      <div className={styles.rightPanel}>
        {selectedClass ? (
          <>
            {/* Class name header */}
            <div className={styles.classNameHeader}>
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
                  color: "var(--dp-text-muted, var(--ol-text-muted))",
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
                  <div className={styles.editorPanel}>
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
          <div className={styles.emptyEditorState}>
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
            <TokenPickerPopover
              ref={tokenPickerRef}
              tokens={tokens}
              onSelectToken={handleSelectToken}
              onUnlinkToken={handleUnlinkToken}
            />
          </div>
        )}
      </div>
    </div>
  );
}
