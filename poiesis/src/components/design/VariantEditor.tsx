// components/design/VariantEditor.tsx
// Variant editor with dimensions tree, base editor, and branch editor.

import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils/id";
import type { VariantBranch, VariantDimension, DesignToken } from "../../core/types/design";
import type { StyleProperty, StyleValueSegment } from "../../core/types/style";
import { parseStyleValue } from "../../services/codegen/design-value-parser";
import { LOCKED_INPUT_STYLE } from "./cards/lock-icon-utils";
import { SizeCard } from "./cards/SizeCard";
import { BackgroundCard } from "./cards/BackgroundCard";
import { BorderCard } from "./cards/BorderCard";
import { RadiusCard } from "./cards/RadiusCard";
import { SpacingCard } from "./cards/SpacingCard";
import { CollapsibleGroup } from "./CollapsibleGroup";
import { PropertyRow } from "./PropertyRow";
import { ScrubInput } from "./ScrubInput";
import { IconButtonGroup } from "./IconButtonGroup";
import { ClassPreview } from "./ClassPreview";
import { TokenSlashPicker } from "./TokenSlashPicker";
import { VariantSidebar } from "./VariantSidebar";
import styles from "./VariantEditor.module.css";

// ── Helpers ──

function segmentsToString(segments: StyleValueSegment[]): string {
  return segments.map((s) => {
    if (s.type === "var-ref") return `token(${s.name})`;
    if (s.type === "text") return s.value;
    return "";
  }).join("");
}

/** Parse a branch key "dimension.value" into { dim, val }, or null for "base". */
function parseBranchKey(key: string): { dim: string; val: string } | null {
  if (key === "base") return null;
  const idx = key.indexOf(".");
  if (idx < 0) return null;
  return { dim: key.slice(0, idx), val: key.slice(idx + 1) };
}

/** Convert kebab-case to camelCase (e.g. "border-radius" → "borderRadius"). */
function kebabToCamel(name: string): string {
  return name.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
}

/** Merge base and branch properties — branch values take priority.
 *  Normalizes all property names to camelCase for consistent lookup. */
function getCombinedProps(
  baseProperties: StyleProperty[],
  branch: { properties: StyleProperty[] } | null,
): StyleProperty[] {
  const normalize = (props: StyleProperty[]) =>
    props.map((p) => ({ ...p, name: kebabToCamel(p.name) }));
  if (!branch) return normalize(baseProperties);
  const merged = new Map<string, StyleProperty>();
  for (const p of normalize(baseProperties)) merged.set(p.name, p);
  for (const p of normalize(branch.properties)) merged.set(p.name, p);
  return Array.from(merged.values());
}

/** Return only the properties in `merged` whose value differs from the base. */
function computeBranchOverrides(
  merged: StyleProperty[],
  baseProperties: StyleProperty[],
): StyleProperty[] {
  const normalized = baseProperties.map((p) => ({ ...p, name: kebabToCamel(p.name) }));
  const baseMap = new Map(normalized.map((p) => [p.name, segmentsToString(p.segments)]));
  return merged.filter((p) => {
    const baseVal = baseMap.get(p.name);
    const mergedVal = segmentsToString(p.segments);
    return baseVal === undefined || baseVal !== mergedVal;
  });
}

/** Pure helper: filter a property list, removing the one with the given name. */
function removePropFromList(
  props: StyleProperty[],
  propName: string,
): StyleProperty[] {
  return props.filter((p) => p.name !== propName);
}

// ── Styles (using --dp-* CSS variables for consistency with TokenEditor/ClassEditor) ──

const INPUT_STYLE: React.CSSProperties = {
  padding: "4px 6px",
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

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, var(--ol-text-muted))",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
  borderRadius: 3,
};

const DASHED_BTN: React.CSSProperties = {
  border: "1px dashed var(--dp-border, var(--ol-border))",
  borderRadius: "var(--dp-radius-sm, 6px)",
  background: "transparent",
  color: "var(--dp-text-muted, var(--ol-text-muted))",
  fontSize: 12,
  padding: "4px 10px",
  cursor: "pointer",
  width: "100%",
  fontFamily: "inherit",
  transition: "color 150ms ease, border-color 150ms ease",
};

const VALUE_PILL: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
  padding: "2px 8px",
  fontSize: 12,
  borderRadius: 12,
  border: "1px solid var(--dp-border, var(--ol-border))",
  background: "var(--dp-surface, var(--ol-surface))",
  color: "var(--dp-text, var(--ol-gray-700))",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const VALUE_PILL_DEFAULT: React.CSSProperties = {
  ...VALUE_PILL,
  background: "var(--dp-accent, var(--ol-color-primary))",
  color: "var(--ol-surface)",
  borderColor: "var(--dp-accent, var(--ol-color-primary))",
};

const BRANCH_TAB: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  cursor: "pointer",
  border: "none",
  background: "none",
  color: "var(--dp-text-muted, var(--ol-text-muted))",
  borderRadius: 4,
  fontFamily: "var(--dp-font-mono, monospace)",
};

const BRANCH_TAB_ACTIVE: React.CSSProperties = {
  ...BRANCH_TAB,
  background: "var(--dp-accent-subtle, var(--ol-color-primary-subtle))",
  color: "var(--dp-accent, var(--ol-color-primary))",
  fontWeight: 600,
};

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, var(--ol-text-secondary))",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "12px 0 6px",
};

// ── Property constants (copied from ClassEditor) ──

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

const layoutPropNames = ["display", "position", "flexDirection", "justifyContent", "alignItems", "alignSelf", "gap", "overflow", "flexWrap"];
const typographyPropNames = ["color", "fontFamily", "fontSize", "fontWeight", "lineHeight", "textAlign", "textDecoration", "textTransform", "letterSpacing"];
const layoutPropSet = new Set(layoutPropNames);
const typographyPropSet = new Set(typographyPropNames);

// ── Helpers for property access ──

function getPropValue(props: StyleProperty[], name: string): string {
  // Try exact match, then camelCase variant
  let prop = props.find((p) => p.name === name);
  if (!prop) {
    const camel = kebabToCamel(name);
    prop = props.find((p) => p.name === camel);
  }
  if (!prop || prop.segments.length === 0) return "";
  return prop.segments.map((s) => {
    if (s.type === "var-ref") return `token(${s.name})`;
    if (s.type === "text") return s.value;
    return "";
  }).join("");
}

function isEnumProperty(name: string): boolean {
  return name in ENUM_OPTIONS;
}

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

// ── Tokenized Input (standalone component) ──

function TokenizedInput({
  value,
  onChange,
  onSlash,
  placeholder,
  style,
}: {
  value: string;
  onChange: (value: string) => void;
  onSlash: (inputEl: HTMLInputElement) => void;
  placeholder?: string;
  style?: React.CSSProperties;
}) {
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newVal = e.target.value;
      if (newVal.endsWith("/") && !value.endsWith("/")) {
        onSlash(e.target as HTMLInputElement);
      }
      onChange(newVal);
    },
    [value, onChange, onSlash],
  );
  return <input value={value} onChange={handleChange} style={style} placeholder={placeholder} />;
}

// ── Hook: all VariantEditor state and handlers ──

function useVariantEditorState() {
  // ── Store ──
  const variants = usePoiesisStore((s) => s.activeDesign.variants);
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const addVariant = usePoiesisStore((s) => s.addVariant);
  const updateVariant = usePoiesisStore((s) => s.updateVariant);
  const removeVariant = usePoiesisStore((s) => s.removeVariant);
  const toggleInheritedProp = usePoiesisStore((s) => s.toggleInheritedProp);

  // ── Local state ──
  const [selectedVariantId, setSelectedVariantId] = useState<string | null>(null);
  const [newDimName, setNewDimName] = useState("");
  const [newDimValue, setNewDimValue] = useState("");
  const [activeBranchKey, setActiveBranchKey] = useState<string>("base");
  const [showAddProp, setShowAddProp] = useState(false);
  const [addPropSearch, setAddPropSearch] = useState("");
  const [previewSelections, setPreviewSelections] = useState<Record<string, string>>({});

  const [slashPicker, setSlashPicker] = useState<{
    propName: string;
    filter: string;
    position: { top: number; left: number };
  } | null>(null);

  const addPropRef = useRef<HTMLDivElement>(null);

  // ── Derived data ──

  const selectedVariant = useMemo(
    () => variants.find((v) => v.id === selectedVariantId) ?? null,
    [variants, selectedVariantId],
  );

  const isBranchMode = activeBranchKey !== "base";

  // Generate branch keys from dimensions: ["color.primary", "color.danger", "size.sm", ...]
  const branchKeys = useMemo(() => {
    if (!selectedVariant) return [];
    const keys: string[] = [];
    for (const dim of selectedVariant.dimensions) {
      for (const val of dim.values) {
        keys.push(`${dim.name}.${val}`);
      }
    }
    return keys;
  }, [selectedVariant]);

  // Current branch object (null for base mode)
  const branch = useMemo(() => {
    if (!selectedVariant || !isBranchMode) return null;
    const parsed = parseBranchKey(activeBranchKey);
    if (!parsed) return null;
    return selectedVariant.branches.find(
      (b) => b.dimension === parsed.dim && b.value === parsed.val,
    ) ?? null;
  }, [selectedVariant, isBranchMode, activeBranchKey]);

  // Merged properties for the active editing context
  const currentProps = useMemo(() => {
    if (!selectedVariant) return [];
    const normalize = (props: StyleProperty[]) =>
      props.map((p) => ({ ...p, name: kebabToCamel(p.name) }));
    if (activeBranchKey === "base") return normalize(selectedVariant.baseProperties);
    const parsed = parseBranchKey(activeBranchKey);
    if (!parsed) return [];
    const branch = selectedVariant.branches.find(
      (b) => b.dimension === parsed.dim && b.value === parsed.val,
    );
    return getCombinedProps(selectedVariant.baseProperties, branch ?? null);
  }, [selectedVariant, activeBranchKey]);

  // Preview props for the live preview panel (merged base + branch per selection)
  const previewProps = useMemo(() => {
    if (!selectedVariant) return [];
    const merged = new Map<string, StyleProperty>();
    const normalize = (props: StyleProperty[]) =>
      props.map((p) => ({ ...p, name: kebabToCamel(p.name) }));
    for (const p of normalize(selectedVariant.baseProperties)) merged.set(p.name, p);
    for (const [dim, val] of Object.entries(previewSelections)) {
      const branch = selectedVariant.branches.find(
        (b) => b.dimension === dim && b.value === val,
      );
      if (branch) {
        for (const p of normalize(branch.properties)) merged.set(p.name, p);
      }
    }
    return Array.from(merged.values());
  }, [selectedVariant, previewSelections]);

  // Sync preview selections when the selected variant changes
  useEffect(() => {
    if (selectedVariant) {
      setPreviewSelections({ ...selectedVariant.defaults });
    }
  }, [selectedVariant?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // Default combination display string
  const defaultComboStr = useMemo(() => {
    if (!selectedVariant) return "";
    return Object.entries(selectedVariant.defaults)
      .map(([k, v]) => `${k}=${v}`)
      .join(", ");
  }, [selectedVariant]);

  const layoutSummary = useMemo(() => {
    const parts = currentProps.filter(
      (p) => layoutPropSet.has(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  const typographySummary = useMemo(() => {
    const parts = currentProps.filter(
      (p) => typographyPropSet.has(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  const inheritedProps = selectedVariant?.inheritedProps ?? [];
  const inheritedPropsSet = useMemo(
    () => new Set(selectedVariant?.inheritedProps ?? []),
    [selectedVariant?.inheritedProps],
  );

  // ── Handlers: Add-property popup ──

  const handleOutsideAddProp = useCallback(() => {
    setShowAddProp(false);
    setAddPropSearch("");
  }, []);

  const handleToggleAddProp = useCallback(() => {
    setShowAddProp((v) => !v);
  }, []);

  // ── Handlers: Variant CRUD ──

  const handleSelectVariant = useCallback((id: string) => {
    setSelectedVariantId(id);
    setActiveBranchKey("base");
  }, []);

  const handleAddVariant = useCallback((name: string) => {
    if (!name.trim()) return;
    addVariant(name.trim());
  }, [addVariant]);

  const handleRemoveVariant = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.stopPropagation();
      removeVariant(id);
      if (selectedVariantId === id) setSelectedVariantId(null);
    },
    [removeVariant, selectedVariantId],
  );

  // ── Handlers: Dimensions ──

  const handleAddDimension = useCallback(() => {
    if (!selectedVariant || !newDimName.trim()) return;
    const name = newDimName.trim();
    // Prevent duplicate dimension names
    if (selectedVariant.dimensions.some((d) => d.name === name)) return;
    const dim: VariantDimension = { name, values: [] };
    updateVariant(selectedVariant.id, {
      dimensions: [...selectedVariant.dimensions, dim],
    });
    setNewDimName("");
  }, [selectedVariant, newDimName, updateVariant]);

  const handleRemoveDimension = useCallback(
    (dimName: string) => {
      if (!selectedVariant) return;
      const { [dimName]: _removedDefault, ...restDefaults } = selectedVariant.defaults;
      updateVariant(selectedVariant.id, {
        dimensions: selectedVariant.dimensions.filter((d) => d.name !== dimName),
        defaults: restDefaults,
        branches: selectedVariant.branches.filter((b) => b.dimension !== dimName),
      });
      // If the removed dimension was the active branch, go back to base
      const parsed = parseBranchKey(activeBranchKey);
      if (parsed && parsed.dim === dimName) {
        setActiveBranchKey("base");
      }
    },
    [selectedVariant, activeBranchKey, updateVariant],
  );

  const handleAddDimValue = useCallback(
    (dimName: string) => {
      if (!selectedVariant || !newDimValue.trim()) return;
      const val = newDimValue.trim();
      const dim = selectedVariant.dimensions.find((d) => d.name === dimName);
      if (!dim) return;
      if (dim.values.includes(val)) return; // no duplicates
      updateVariant(selectedVariant.id, {
        dimensions: selectedVariant.dimensions.map((d) =>
          d.name === dimName ? { ...d, values: [...d.values, val] } : d,
        ),
      });
      setNewDimValue("");
    },
    [selectedVariant, newDimValue, updateVariant],
  );

  const handleRemoveDimValue = useCallback(
    (dimName: string, value: string) => {
      if (!selectedVariant) return;
      const dim = selectedVariant.dimensions.find((d) => d.name === dimName);
      if (!dim) return;

      // Also clean up default if it was this value
      const newDefaults = { ...selectedVariant.defaults };
      if (newDefaults[dimName] === value) {
        delete newDefaults[dimName];
      }

      updateVariant(selectedVariant.id, {
        dimensions: selectedVariant.dimensions.map((d) =>
          d.name === dimName ? { ...d, values: d.values.filter((v) => v !== value) } : d,
        ),
        defaults: newDefaults,
        branches: selectedVariant.branches.filter(
          (b) => !(b.dimension === dimName && b.value === value),
        ),
      });

      // If the active branch was this dimension+value, go back to base
      const parsed = parseBranchKey(activeBranchKey);
      if (parsed && parsed.dim === dimName && parsed.val === value) {
        setActiveBranchKey("base");
      }
    },
    [selectedVariant, activeBranchKey, updateVariant],
  );

  const handleSetDefault = useCallback(
    (dimName: string, value: string) => {
      if (!selectedVariant) return;
      updateVariant(selectedVariant.id, {
        defaults: { ...selectedVariant.defaults, [dimName]: value },
      });
    },
    [selectedVariant, updateVariant],
  );

  // ── Handlers: Branch selection ──

  const handleSelectBranch = useCallback((key: string) => {
    setActiveBranchKey(key);
  }, []);

  // ── Handlers: Properties ──

  /** Handle props change from a card — diffs against base to determine overrides. */
  const handleCardPropsChange = useCallback(
    (newProps: StyleProperty[]) => {
      if (!selectedVariant) return;

      if (activeBranchKey === "base") {
        updateVariant(selectedVariant.id, { baseProperties: newProps });
        return;
      }

      const parsed = parseBranchKey(activeBranchKey);
      if (!parsed) return;

      const overrides = computeBranchOverrides(newProps, selectedVariant.baseProperties);
      const branch = selectedVariant.branches.find(
        (b) => b.dimension === parsed.dim && b.value === parsed.val,
      );

      if (branch) {
        updateVariant(selectedVariant.id, {
          branches: selectedVariant.branches.map((b) =>
            b.id === branch.id ? { ...b, properties: overrides } : b,
          ),
        });
      } else if (overrides.length > 0) {
        const newBranch: VariantBranch = {
          id: uid("vb"),
          dimension: parsed.dim,
          value: parsed.val,
          properties: overrides,
        };
        updateVariant(selectedVariant.id, {
          branches: [...selectedVariant.branches, newBranch],
        });
      }
    },
    [selectedVariant, activeBranchKey, updateVariant],
  );

  /** Handle single property change from a group row — builds the full props list and delegates to card handler. */
  const handleSinglePropChange = useCallback(
    (name: string, value: string) => {
      if (!selectedVariant) return;
      const existing = currentProps.find((p) => p.name === name);
      let newProps: StyleProperty[];
      if (existing) {
        newProps = currentProps.map((p) =>
          p.name === name
            ? { ...p, segments: parseStyleValue(value) }
            : p,
        );
      } else {
        newProps = [
          ...currentProps,
          { id: uid("prop"), name, segments: parseStyleValue(value) },
        ];
      }
      handleCardPropsChange(newProps);
    },
    [selectedVariant, currentProps, handleCardPropsChange],
  );

  const handleSlashTrigger = useCallback(
    (propName: string, el: HTMLElement) => {
      const rect = el.getBoundingClientRect();
      setSlashPicker({
        propName,
        filter: "",
        position: { top: rect.bottom + 4, left: rect.left },
      });
    },
    [],
  );

  const handleTokenSelect = useCallback(
    (tokenName: string) => {
      if (!slashPicker) return;
      const currentVal = getPropValue(currentProps, slashPicker.propName);
      let newVal: string;
      if (currentVal.endsWith("/")) {
        // Typed "/" in TokenizedInput — replace the trailing "/"
        newVal = currentVal.slice(0, -1) + `token(${tokenName}) `;
      } else if (currentVal.startsWith("token(")) {
        // Clicked an existing token ref — replace it entirely
        newVal = `token(${tokenName})`;
      } else {
        // Opened from ScrubInput (empty or numeric) — set the token
        newVal = `token(${tokenName})`;
      }
      handleSinglePropChange(slashPicker.propName, newVal);
      setSlashPicker(null);
    },
    [slashPicker, currentProps, handleSinglePropChange],
  );

  const handleRemoveProperty = useCallback(
    (propName: string) => {
      if (!selectedVariant) return;

      if (activeBranchKey === "base") {
        updateVariant(selectedVariant.id, {
          baseProperties: removePropFromList(selectedVariant.baseProperties, propName),
        });
      } else {
        const parsed = parseBranchKey(activeBranchKey);
        if (!parsed) return;
        const branch = selectedVariant.branches.find(
          (b) => b.dimension === parsed.dim && b.value === parsed.val,
        );
        if (!branch) return;
        updateVariant(selectedVariant.id, {
          branches: selectedVariant.branches.map((b) =>
            b.id === branch.id
              ? { ...b, properties: removePropFromList(b.properties, propName) }
              : b,
          ),
        });
      }
    },
    [selectedVariant, activeBranchKey, updateVariant],
  );

  const handleSelectAddProp = useCallback((name: string) => {
    handleSinglePropChange(name, "");
    setShowAddProp(false);
    setAddPropSearch("");
  }, [handleSinglePropChange]);

  return {
    variants, tokens, selectedVariantId, newDimName, newDimValue,
    activeBranchKey, showAddProp, addPropSearch, previewSelections,
    slashPicker, addPropRef, selectedVariant, isBranchMode,
    branchKeys, branch, currentProps, previewProps,
    defaultComboStr, layoutSummary, typographySummary,
    inheritedProps, inheritedPropsSet,
    toggleInheritedProp,
    setNewDimName, setNewDimValue, setAddPropSearch, setPreviewSelections, setSlashPicker,
    handleSelectVariant, handleAddVariant, handleRemoveVariant,
    handleAddDimension, handleRemoveDimension, handleAddDimValue,
    handleRemoveDimValue, handleSetDefault, handleSelectBranch,
    handleCardPropsChange, handleSinglePropChange, handleSlashTrigger,
    handleTokenSelect, handleRemoveProperty,
    handleOutsideAddProp, handleToggleAddProp, handleSelectAddProp,
  };
}

// ── Extracted sub-components ──

function DefaultComboDisplay({
  selectedVariant,
  defaultComboStr,
}: {
  selectedVariant: { readonly defaults: Record<string, string> } | null;
  defaultComboStr: string;
}) {
  if (!selectedVariant || Object.keys(selectedVariant.defaults).length === 0) return null;
  return (
    <div
      style={{
        fontSize: 12,
        color: "var(--dp-text-secondary, var(--ol-text-secondary))",
        padding: "6px 8px",
        background: "var(--dp-bg, var(--ol-gray-50))",
        borderRadius: "var(--dp-radius-sm, 6px)",
        marginBottom: 12,
        border: "1px solid var(--dp-border, var(--ol-border))",
      }}
    >
      <span style={{ fontWeight: 500, color: "var(--dp-text, var(--ol-gray-700))" }}>默认组合: </span>
      {defaultComboStr}
    </div>
  );
}

function DimensionsSection({
  selectedVariant,
  newDimValue,
  newDimName,
  onNewDimValueChange,
  onNewDimNameChange,
  onAddDimension,
  onRemoveDimension,
  onAddDimValue,
  onRemoveDimValue,
  onSetDefault,
}: {
  selectedVariant: {
    readonly dimensions: readonly VariantDimension[];
    readonly defaults: Readonly<Record<string, string>>;
  } | null;
  newDimValue: string;
  newDimName: string;
  onNewDimValueChange: (v: string) => void;
  onNewDimNameChange: (v: string) => void;
  onAddDimension: () => void;
  onRemoveDimension: (name: string) => void;
  onAddDimValue: (name: string) => void;
  onRemoveDimValue: (name: string, value: string) => void;
  onSetDefault: (name: string, value: string) => void;
}) {
  if (!selectedVariant) return null;

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={SECTION_HEADER}>Dimensions</div>
      {selectedVariant.dimensions.length === 0 && (
        <div style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))", marginBottom: 6 }}>
          No dimensions yet. Add your first dimension below.
        </div>
      )}
      {selectedVariant.dimensions.map((dim) => {
        const defaultValue = selectedVariant.defaults[dim.name];
        return (
          <div
            key={dim.name}
            style={{
              marginBottom: 8,
              padding: "6px 8px",
              background: "var(--dp-bg, var(--ol-gray-50))",
              borderRadius: 6,
              border: "1px solid var(--dp-border, var(--ol-border))",
            }}
          >
            {/* Dimension header */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                marginBottom: 4,
              }}
            >
              <span style={{ fontSize: 12, fontWeight: 600, color: "var(--dp-text, var(--ol-gray-700))" }}>
                {dim.name}
              </span>
              <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))" }}>
                ({dim.values.length} values)
              </span>
              <div style={{ flex: 1 }} />
              <button type="button"
                onClick={() => onRemoveDimension(dim.name)}
                style={REMOVE_BTN}
                title="Remove dimension"
              >
                &times;
              </button>
            </div>

            {/* Value pills */}
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 4 }}>
              {dim.values.map((val) => {
                const isDefault = val === defaultValue;
                return (
                  <button
                    type="button"
                    key={val}
                    onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'remove-dim-value') { e.stopPropagation(); onRemoveDimValue(dim.name, val); } else { onSetDefault(dim.name, val); } }}
                    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSetDefault(dim.name, val); } }}
                    style={isDefault ? VALUE_PILL_DEFAULT : VALUE_PILL}
                    title={
                      isDefault
                        ? `Default for ${dim.name}`
                        : `Click to set as default`
                    }
                  >
                    {val}
                    <span data-action="remove-dim-value" title="Remove value" style={{
                      border: "none",
                      background: "none",
                      cursor: "pointer",
                      color: isDefault ? "rgba(255,255,255,0.7)" : "var(--dp-text-muted, var(--ol-text-muted))",
                      fontSize: 12,
                      padding: 0,
                      lineHeight: 1,
                      marginLeft: 2,
                    }}>
                      &times;
                    </span>
                  </button>
                );
              })}
            </div>

            {/* Add value input */}
            <div style={{ display: "flex", gap: 4 }}>
              <input
                aria-label="Add dimension value"
                value={newDimValue}
                onChange={(e) => onNewDimValueChange(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") onAddDimValue(dim.name);
                }}
                placeholder="+ value"
                style={{
                  ...INPUT_STYLE,
                  width: 120,
                  fontSize: 12,
                }}
              />
              <button type="button"
                onClick={() => onAddDimValue(dim.name)}
                disabled={!newDimValue.trim()}
                style={{
                  padding: "2px 8px",
                  fontSize: 12,
                  border: "1px solid var(--dp-border, var(--ol-border))",
                  borderRadius: "var(--dp-radius-sm, 6px)",
                  background: newDimValue.trim() ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-bg, var(--ol-gray-50))",
                  color: newDimValue.trim() ? "var(--ol-surface)" : "var(--dp-text-muted, var(--ol-text-muted))",
                  cursor: newDimValue.trim() ? "pointer" : "default",
                }}
              >
                +
              </button>
            </div>
          </div>
        );
      })}

      {/* Add dimension input */}
      {selectedVariant.dimensions.length > 0 && <div style={{ height: 4 }} />}
      <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
        <input
          aria-label="Add dimension"
          value={newDimName}
          onChange={(e) => onNewDimNameChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onAddDimension();
          }}
          placeholder='+ 添加维度 (e.g. "color", "size")'
          style={{ ...INPUT_STYLE, flex: 1, minWidth: 0, fontSize: 12 }}
        />
        <button type="button"
          onClick={onAddDimension}
          disabled={!newDimName.trim()}
          style={{
            padding: "3px 8px",
            fontSize: 12,
            border: "none",
            borderRadius: 4,
            background: newDimName.trim() ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-border, var(--ol-border))",
            color: newDimName.trim() ? "var(--ol-surface)" : "var(--dp-text-muted, var(--ol-text-muted))",
            cursor: newDimName.trim() ? "pointer" : "default",
            whiteSpace: "nowrap",
          }}
        >
          + 添加维度
        </button>
      </div>
    </div>
  );
}

function BranchTabs({
  selectedVariant,
  branchKeys,
  activeBranchKey,
  onSelectBranch,
}: {
  selectedVariant: unknown;
  branchKeys: string[];
  activeBranchKey: string;
  onSelectBranch: (key: string) => void;
}) {
  if (!selectedVariant) return null;
  const branchesExist = branchKeys.length > 0;

  return (
    <div className={styles.branchTabsBar}>
      <button type="button"
        onClick={() => onSelectBranch("base")}
        style={activeBranchKey === "base" ? BRANCH_TAB_ACTIVE : BRANCH_TAB}
      >
        base
      </button>
      {branchKeys.map((key) => (
        <button type="button"
          key={key}
          onClick={() => onSelectBranch(key)}
          style={activeBranchKey === key ? BRANCH_TAB_ACTIVE : BRANCH_TAB}
        >
          {key}
        </button>
      ))}
      {!branchesExist && (
        <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))", marginLeft: 4 }}>
          (add dimension values above)
        </span>
      )}
    </div>
  );
}

function AddPropertyDropdown({
  showAddProp,
  addPropSearch,
  onToggleShow,
  onSearchChange,
  onSelectProp,
  addPropRef,
}: {
  showAddProp: boolean;
  addPropSearch: string;
  onToggleShow: () => void;
  onSearchChange: (v: string) => void;
  onSelectProp: (name: string) => void;
  addPropRef: React.RefObject<HTMLDivElement | null>;
}) {
  const filtered = ALL_PROPERTY_NAMES.filter(
    (n) => n.toLowerCase().includes(addPropSearch.toLowerCase()),
  );

  return (
    <div style={{ position: "relative", margin: "4px 8px" }}>
      <button type="button"
        onClick={onToggleShow}
        style={DASHED_BTN}
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
            onChange={(e) => onSearchChange(e.target.value)}
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
              onClick={() => onSelectProp(name)}
              className={styles.addPropItem}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLElement).style.background = "var(--dp-surface-hover, var(--ol-color-primary-subtle))";
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLElement).style.background = "transparent";
              }}
            >
              <span style={{ color: "var(--dp-text-muted, var(--ol-text-muted))", marginRight: 4 }}>+</span>
              {PROPERTY_LABELS[name] || name}
            </button>
          ))}
          {filtered.length === 0 && (
            <div
              style={{
                padding: 8,
                fontSize: 12,
                color: "var(--dp-text-muted, var(--ol-text-muted))",
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
}

function CardsEditor({
  selectedVariant,
  currentProps,
  onCardPropsChange,
  onSlashTrigger,
  inheritedProps,
  isBranchMode,
  onToggleInheritedProp,
  branch,
  onSinglePropChange,
  onRemoveProperty,
  layoutSummary,
  typographySummary,
  tokens,
  showAddProp,
  addPropSearch,
  onToggleShowAddProp,
  onAddPropSearchChange,
  addPropRef,
  onSelectAddProp,
}: {
  selectedVariant: {
    readonly id: string;
    readonly baseProperties: StyleProperty[];
  } | null;
  currentProps: StyleProperty[];
  onCardPropsChange: (props: StyleProperty[]) => void;
  onSlashTrigger: (propName: string, el: HTMLElement) => void;
  inheritedProps: string[];
  isBranchMode: boolean;
  onToggleInheritedProp: (id: string, name: string) => void;
  branch: VariantBranch | null;
  onSinglePropChange: (name: string, value: string) => void;
  onRemoveProperty: (name: string) => void;
  layoutSummary: string;
  typographySummary: string;
  tokens: DesignToken[];
  showAddProp: boolean;
  addPropSearch: string;
  onToggleShowAddProp: () => void;
  onAddPropSearchChange: (v: string) => void;
  addPropRef: React.RefObject<HTMLDivElement | null>;
  onSelectAddProp: (name: string) => void;
}) {
  const inheritedPropsSet = useMemo(() => new Set(inheritedProps), [inheritedProps]);

  if (!selectedVariant) return null;

  return (
    <div>
      {/* Visual editing cards */}
      <SizeCard
        properties={currentProps}
        onChange={onCardPropsChange}
        onPropSlash={onSlashTrigger}
        inheritedProps={inheritedProps}
        onToggleInherited={isBranchMode ? undefined : (name) => onToggleInheritedProp(selectedVariant.id, name)}
        readOnly={isBranchMode}
      />
      <BackgroundCard
        properties={currentProps}
        tokens={tokens}
        onChange={onCardPropsChange}
        inheritedProps={inheritedProps}
        onToggleInherited={isBranchMode ? undefined : (name) => onToggleInheritedProp(selectedVariant.id, name)}
        readOnly={isBranchMode}
      />
      <BorderCard
        properties={currentProps}
        tokens={tokens}
        onChange={onCardPropsChange}
        inheritedProps={inheritedProps}
        onToggleInherited={isBranchMode ? undefined : (name) => onToggleInheritedProp(selectedVariant.id, name)}
        readOnly={isBranchMode}
      />
      <RadiusCard
        properties={currentProps}
        onChange={onCardPropsChange}
        onPropSlash={onSlashTrigger}
        inheritedProps={inheritedProps}
        onToggleInherited={isBranchMode ? undefined : (name) => onToggleInheritedProp(selectedVariant.id, name)}
        readOnly={isBranchMode}
      />
      <SpacingCard
        properties={currentProps}
        onChange={onCardPropsChange}
        onPropSlash={onSlashTrigger}
        inheritedProps={inheritedProps}
        onToggleInherited={isBranchMode ? undefined : (name) => onToggleInheritedProp(selectedVariant.id, name)}
        readOnly={isBranchMode}
      />

      {/* Layout group for remaining layout properties */}
      <CollapsibleGroup title="布局 Layout" summary={layoutSummary} defaultOpen={false}>
        {layoutPropNames.map((name) => {
          const value = getPropValue(currentProps, name);
          const label = PROPERTY_LABELS[name] || name;
          const isInherited = inheritedPropsSet.has(name);

          // Inherited prop in branch mode: read-only
          if (isBranchMode && isInherited) {
            const baseValue = getPropValue(selectedVariant.baseProperties, name);
            return (
              <PropertyRow key={name} label={label} inherited={true}>
                <div style={{ ...INPUT_STYLE, background: "var(--dp-bg, var(--ol-gray-50))", color: "var(--dp-text-muted, var(--ol-text-muted))", cursor: "not-allowed", display: "flex", alignItems: "center", fontSize: 12 }}>
                  ← base: {baseValue || "—"}
                </div>
              </PropertyRow>
            );
          }

          let input: React.ReactNode;

          if (name === "flexDirection") {
            input = (
              <IconButtonGroup
                options={[
                  { value: "row", label: "→", title: "row" },
                  { value: "column", label: "↓", title: "column" },
                  { value: "row-reverse", label: "←", title: "row-reverse" },
                  { value: "column-reverse", label: "↑", title: "column-reverse" },
                ]}
                value={(value || "row") as "row" | "column" | "row-reverse" | "column-reverse"}
                onChange={(v) => onSinglePropChange(name, v as string)}
              />
            );
          } else if (name === "gap") {
            input = (
              <ScrubInput
                value={parseFloat(value) || 0}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => onSinglePropChange(name, `${v}px`)}
              />
            );
          } else if (isEnumProperty(name)) {
            input = (
              <select
                aria-label={label}
                value={value}
                onChange={(e) => onSinglePropChange(name, e.target.value)}
                style={SELECT_STYLE}
              >
                <option value="">—</option>
                {ENUM_OPTIONS[name]?.map((opt) => (
                  <option key={opt} value={opt}>{opt}</option>
                )) ?? null}
              </select>
            );
          } else {
            input = (
              <TokenizedInput
                value={value}
                onChange={(v) => onSinglePropChange(name, v)}
                onSlash={(el) => onSlashTrigger(name, el)}
                placeholder="value"
                style={INPUT_STYLE}
              />
            );
          }

          const isLockedInBase = !isBranchMode && !isInherited;
          return (
            <PropertyRow
              key={name}
              label={label}
              showOverrideDot={isBranchMode && !!branch?.properties.some((p) => p.name === name)}
              onRemove={() => onRemoveProperty(name)}
            >
              <span style={isLockedInBase ? LOCKED_INPUT_STYLE : undefined}>
                {input}
              </span>
              {!isBranchMode && (
                <button type="button"
                  onClick={() => onToggleInheritedProp(selectedVariant.id, name)}
                  title={isInherited ? "已解锁（分支只读）— 点击封锁" : "已封锁（分支可覆盖）— 点击解锁"}
                  style={{ border: "none", background: "none", cursor: "pointer", fontSize: 12, padding: "0 4px", color: isInherited ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-text-muted, var(--ol-text-muted))" }}
                >{isInherited ? "🔓" : "🔒"}</button>
              )}
            </PropertyRow>
          );
        })}
      </CollapsibleGroup>

      {/* Typography group */}
      <CollapsibleGroup title="文字 Typography" summary={typographySummary} defaultOpen={false}>
        {typographyPropNames.map((name) => {
          const value = getPropValue(currentProps, name);
          const label = PROPERTY_LABELS[name] || name;
          const isInherited = inheritedPropsSet.has(name);

          // Inherited prop in branch mode: read-only
          if (isBranchMode && isInherited) {
            const baseValue = getPropValue(selectedVariant.baseProperties, name);
            return (
              <PropertyRow key={name} label={label} inherited={true}>
                <div style={{ ...INPUT_STYLE, background: "var(--dp-bg, var(--ol-gray-50))", color: "var(--dp-text-muted, var(--ol-text-muted))", cursor: "not-allowed", display: "flex", alignItems: "center", fontSize: 12 }}>
                  ← base: {baseValue || "—"}
                </div>
              </PropertyRow>
            );
          }

          let input: React.ReactNode;

          if (["fontSize", "lineHeight", "letterSpacing"].includes(name)) {
            const unit = name === "fontSize" ? "px" : (name === "letterSpacing" ? "px" : "");
            input = (
              <ScrubInput
                value={parseFloat(value) || 0}
                unit={unit}
                min={0}
                max={999}
                onChange={(v) => onSinglePropChange(name, unit ? `${v}${unit}` : `${v}`)}
              />
            );
          } else if (isEnumProperty(name)) {
            input = (
              <select
                aria-label={label}
                value={value}
                onChange={(e) => onSinglePropChange(name, e.target.value)}
                style={SELECT_STYLE}
              >
                <option value="">—</option>
                {ENUM_OPTIONS[name]?.map((opt) => (
                  <option key={opt} value={opt}>{opt}</option>
                )) ?? null}
              </select>
            );
          } else {
            input = (
              <TokenizedInput
                value={value}
                onChange={(v) => onSinglePropChange(name, v)}
                onSlash={(el) => onSlashTrigger(name, el)}
                placeholder="value"
                style={INPUT_STYLE}
              />
            );
          }

          const isLockedInBase = !isBranchMode && !isInherited;
          return (
            <PropertyRow
              key={name}
              label={label}
              showOverrideDot={isBranchMode && !!branch?.properties.some((p) => p.name === name)}
              onRemove={() => onRemoveProperty(name)}
            >
              <span style={isLockedInBase ? LOCKED_INPUT_STYLE : undefined}>
                {input}
              </span>
              {!isBranchMode && (
                <button type="button"
                  onClick={() => onToggleInheritedProp(selectedVariant.id, name)}
                  title={isInherited ? "已解锁（分支只读）— 点击封锁" : "已封锁（分支可覆盖）— 点击解锁"}
                  style={{ border: "none", background: "none", cursor: "pointer", fontSize: 12, padding: "0 4px", color: isInherited ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-text-muted, var(--ol-text-muted))" }}
                >{isInherited ? "🔓" : "🔒"}</button>
              )}
            </PropertyRow>
          );
        })}
      </CollapsibleGroup>

      <AddPropertyDropdown
        showAddProp={showAddProp}
        addPropSearch={addPropSearch}
        onToggleShow={onToggleShowAddProp}
        onSearchChange={onAddPropSearchChange}
        onSelectProp={onSelectAddProp}
        addPropRef={addPropRef}
      />
    </div>
  );
}

function PreviewPanel({
  selectedVariant,
  previewSelections,
  onPreviewSelectionsChange,
  previewProps,
  tokens,
}: {
  selectedVariant: {
    readonly dimensions: readonly VariantDimension[];
    readonly name: string;
  } | null;
  previewSelections: Record<string, string>;
  onPreviewSelectionsChange: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  previewProps: StyleProperty[];
  tokens: DesignToken[];
}) {
  if (!selectedVariant) return null;
  return (
    <div className={styles.previewPanel}>
      {/* Header */}
      <div className={styles.previewHeader}
      >
        Preview
      </div>

      {/* Dimension combination selectors */}
      {selectedVariant.dimensions.map((dim) => (
        <div key={dim.name} className={styles.previewSelectRow}>
          <span className={styles.previewSelectLabel}>
            {dim.name}
          </span>
          <select
            aria-label={dim.name}
            value={previewSelections[dim.name] || ""}
            onChange={(e) =>
              onPreviewSelectionsChange((prev) => ({
                ...prev,
                [dim.name]: e.target.value,
              }))
            }
            className={styles.previewSelect}
          >
            <option value="">—</option>
            {dim.values.map((val) => (
              <option key={val} value={val}>
                {val}
              </option>
            ))}
          </select>
        </div>
      ))}

      <ClassPreview
        baseProperties={previewProps}
        pseudoStates={[]}
        className={selectedVariant.name}
        tokens={tokens}
      />
    </div>
  );
}

// ── Component ──

export function VariantEditor() {
  const {
    variants, tokens, selectedVariantId, newDimName, newDimValue,
    activeBranchKey, showAddProp, addPropSearch, previewSelections,
    slashPicker, addPropRef, selectedVariant, isBranchMode,
    branchKeys, branch, currentProps, previewProps,
    defaultComboStr, layoutSummary, typographySummary,
    inheritedProps, toggleInheritedProp,
    setNewDimName, setNewDimValue, setAddPropSearch, setPreviewSelections, setSlashPicker,
    handleSelectVariant, handleAddVariant, handleRemoveVariant,
    handleAddDimension, handleRemoveDimension, handleAddDimValue,
    handleRemoveDimValue, handleSetDefault, handleSelectBranch,
    handleCardPropsChange, handleSinglePropChange, handleSlashTrigger,
    handleTokenSelect, handleRemoveProperty,
    handleOutsideAddProp, handleToggleAddProp, handleSelectAddProp,
  } = useVariantEditorState();

  // ── Close add-property popup on outside click ──
  useOutsideClick(addPropRef, showAddProp, handleOutsideAddProp);

  // ── Main render ──

  return (
    <div
      style={{
        display: "flex",
        height: "100%",
        fontSize: 13,
        color: "var(--dp-text, var(--ol-gray-700))",
        position: "relative",
      }}
    >
      <VariantSidebar
        variants={variants}
        selectedVariantId={selectedVariantId}
        onSelectVariant={handleSelectVariant}
        onAddVariant={handleAddVariant}
        onRemoveVariant={handleRemoveVariant}
      />

      <div
        style={{
          flex: 1,
          display: "flex",
          overflow: "hidden",
          minHeight: 0,
        }}
      >
        <div
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {selectedVariant ? (
            <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
              {/* Variant name header */}
              <div
                style={{
                  fontSize: 14,
                  fontWeight: 600,
                  color: "var(--dp-text, var(--ol-gray-700))",
                  marginBottom: 8,
                }}
              >
                {selectedVariant.name}
              </div>

              {/* Default combination */}
              <DefaultComboDisplay
                selectedVariant={selectedVariant}
                defaultComboStr={defaultComboStr}
              />

              {/* Dimensions */}
              <DimensionsSection
                selectedVariant={selectedVariant}
                newDimValue={newDimValue}
                newDimName={newDimName}
                onNewDimValueChange={setNewDimValue}
                onNewDimNameChange={setNewDimName}
                onAddDimension={handleAddDimension}
                onRemoveDimension={handleRemoveDimension}
                onAddDimValue={handleAddDimValue}
                onRemoveDimValue={handleRemoveDimValue}
                onSetDefault={handleSetDefault}
              />

              {/* Branch tabs */}
              <BranchTabs
                selectedVariant={selectedVariant}
                branchKeys={branchKeys}
                activeBranchKey={activeBranchKey}
                onSelectBranch={handleSelectBranch}
              />

              {/* Cards-based property editor */}
              <CardsEditor
                selectedVariant={selectedVariant}
                currentProps={currentProps}
                onCardPropsChange={handleCardPropsChange}
                onSlashTrigger={handleSlashTrigger}
                inheritedProps={inheritedProps}
                isBranchMode={isBranchMode}
                onToggleInheritedProp={toggleInheritedProp}
                branch={branch}
                onSinglePropChange={handleSinglePropChange}
                onRemoveProperty={handleRemoveProperty}
                layoutSummary={layoutSummary}
                typographySummary={typographySummary}
                tokens={tokens}
                showAddProp={showAddProp}
                addPropSearch={addPropSearch}
                onToggleShowAddProp={handleToggleAddProp}
                onAddPropSearchChange={setAddPropSearch}
                addPropRef={addPropRef}
                onSelectAddProp={handleSelectAddProp}
              />
            </div>
          ) : (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                color: "var(--dp-text-muted, var(--ol-text-muted))",
                fontSize: 13,
              }}
            >
              Select a variant to edit
            </div>
          )}
        </div>
        {selectedVariant && (
          <PreviewPanel
            selectedVariant={selectedVariant}
            previewSelections={previewSelections}
            onPreviewSelectionsChange={setPreviewSelections}
            previewProps={previewProps}
            tokens={tokens}
          />
        )}
      </div>
      {slashPicker && (
        <TokenSlashPicker
          tokens={tokens}
          filter={slashPicker.filter}
          onSelect={handleTokenSelect}
          onClose={() => setSlashPicker(null)}
          position={slashPicker.position}
        />
      )}
    </div>
  );
}
