// components/design/VariantEditor.tsx
// Variant editor with dimensions tree, base editor, and branch editor.

import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils";
import type { VariantBranch, VariantDimension } from "../../core/types/design";
import type { StyleProperty, StyleValueSegment } from "../../core/types/style";
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

// ── Helpers ──

function segmentsToString(segments: StyleValueSegment[]): string {
  return segments.map((s) => (s.type === "text" ? s.value : "")).join("");
}

function stringToSegments(value: string): StyleValueSegment[] {
  return value ? [{ type: "text" as const, value }] : [];
}

/** Parse a branch key "dimension.value" into { dim, val }, or null for "base". */
function parseBranchKey(key: string): { dim: string; val: string } | null {
  if (key === "base") return null;
  const idx = key.indexOf(".");
  if (idx < 0) return null;
  return { dim: key.slice(0, idx), val: key.slice(idx + 1) };
}

/** Merge base and branch properties — branch values take priority. */
function getCombinedProps(
  baseProperties: StyleProperty[],
  branch: { properties: StyleProperty[] } | null,
): StyleProperty[] {
  if (!branch) return baseProperties;
  const merged = new Map<string, StyleProperty>();
  for (const p of baseProperties) merged.set(p.name, p);
  for (const p of branch.properties) merged.set(p.name, p);
  return Array.from(merged.values());
}

/** Return only the properties in `merged` whose value differs from the base. */
function computeBranchOverrides(
  merged: StyleProperty[],
  baseProperties: StyleProperty[],
): StyleProperty[] {
  const baseMap = new Map(baseProperties.map((p) => [p.name, segmentsToString(p.segments)]));
  return merged.filter((p) => {
    const baseVal = baseMap.get(p.name);
    const mergedVal = segmentsToString(p.segments);
    return baseVal === undefined || baseVal !== mergedVal;
  });
}

// ── Styles (using --dp-* CSS variables for consistency with TokenEditor/ClassEditor) ──

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
  padding: "10px 8px 6px",
};

const ITEM_BASE: React.CSSProperties = {
  padding: "6px 12px",
  fontSize: 13,
  cursor: "pointer",
  color: "var(--dp-text-secondary, #6b6893)",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  borderRadius: "var(--dp-radius-sm, 6px)",
  margin: "1px 6px",
  transition: "background 100ms ease, color 100ms ease",
};

const ITEM_ACTIVE: React.CSSProperties = {
  ...ITEM_BASE,
  background: "var(--dp-accent-subtle, #eef0ff)",
  color: "var(--dp-accent, #6366f1)",
  fontWeight: 600,
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "4px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "none",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "3px 4px",
  fontSize: 10,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "none",
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

const DASHED_BTN: React.CSSProperties = {
  border: "1px dashed var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  background: "transparent",
  color: "var(--dp-text-muted, #a09db8)",
  fontSize: 11,
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
  border: "1px solid var(--dp-border, #e8e5f2)",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const VALUE_PILL_DEFAULT: React.CSSProperties = {
  ...VALUE_PILL,
  background: "var(--dp-accent, #6366f1)",
  color: "#fff",
  borderColor: "var(--dp-accent, #6366f1)",
};

const BRANCH_TAB: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  cursor: "pointer",
  border: "none",
  background: "none",
  color: "var(--dp-text-muted, #a09db8)",
  borderRadius: 4,
  fontFamily: "var(--dp-font-mono, monospace)",
};

const BRANCH_TAB_ACTIVE: React.CSSProperties = {
  ...BRANCH_TAB,
  background: "var(--dp-accent-subtle, #eef0ff)",
  color: "var(--dp-accent, #6366f1)",
  fontWeight: 600,
};

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, #6b6893)",
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

// ── Helpers for property access ──

function getPropValue(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
  if (!prop || prop.segments.length === 0) return "";
  const seg = prop.segments[0];
  return seg.type === "text" ? seg.value : "";
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

// ── Component ──

export function VariantEditor() {
  // ── Store ──
  const variants = usePoiesisStore((s) => s.activeDesign.variants);
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const addVariant = usePoiesisStore((s) => s.addVariant);
  const updateVariant = usePoiesisStore((s) => s.updateVariant);
  const removeVariant = usePoiesisStore((s) => s.removeVariant);

  // ── Local state ──
  const [selectedVariantId, setSelectedVariantId] = useState<string | null>(null);
  const [newVariantName, setNewVariantName] = useState("");
  const [newDimName, setNewDimName] = useState("");
  const [newDimValue, setNewDimValue] = useState("");
  const [activeBranchKey, setActiveBranchKey] = useState<string>("base");
  const [showAddProp, setShowAddProp] = useState(false);
  const [addPropSearch, setAddPropSearch] = useState("");
  const [previewSelections, setPreviewSelections] = useState<Record<string, string>>({});

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
    if (activeBranchKey === "base") return selectedVariant.baseProperties;
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
    for (const p of selectedVariant.baseProperties) merged.set(p.name, p);
    for (const [dim, val] of Object.entries(previewSelections)) {
      const branch = selectedVariant.branches.find(
        (b) => b.dimension === dim && b.value === val,
      );
      if (branch) {
        for (const p of branch.properties) merged.set(p.name, p);
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
      (p) => layoutPropNames.includes(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  const typographySummary = useMemo(() => {
    const parts = currentProps.filter(
      (p) => typographyPropNames.includes(p.name) && segmentsToString(p.segments),
    );
    return parts.length > 0
      ? parts.map((p) => `${p.name}:${segmentsToString(p.segments)}`).join(", ")
      : "—";
  }, [currentProps]);

  // ── Close add-property popup on outside click ──

  useOutsideClick(addPropRef, showAddProp, () => {
    setShowAddProp(false);
    setAddPropSearch("");
  });

  // ── Handlers: Variant CRUD ──

  const handleSelectVariant = useCallback((id: string) => {
    setSelectedVariantId(id);
    setActiveBranchKey("base");
  }, []);

  const handleAddVariant = useCallback(() => {
    const name = newVariantName.trim();
    if (!name) return;
    addVariant(name);
    setNewVariantName("");
  }, [newVariantName, addVariant]);

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

  const removePropFromList = useCallback(
    (props: StyleProperty[], propName: string): StyleProperty[] =>
      props.filter((p) => p.name !== propName),
    [],
  );

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
            ? { ...p, segments: stringToSegments(value) }
            : p,
        );
      } else {
        newProps = [
          ...currentProps,
          { id: uid("prop"), name, segments: stringToSegments(value) },
        ];
      }
      handleCardPropsChange(newProps);
    },
    [selectedVariant, currentProps, handleCardPropsChange],
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
    [selectedVariant, activeBranchKey, updateVariant, removePropFromList],
  );

  // ── Render: Sidebar ──

  const renderSidebar = () => (
    <div style={SIDEBAR_STYLE}>
      <div style={SIDEBAR_HEADER}>Variants ({variants.length})</div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {variants.map((v) => (
          <div
            key={v.id}
            onClick={() => handleSelectVariant(v.id)}
            style={v.id === selectedVariantId ? ITEM_ACTIVE : ITEM_BASE}
          >
            <span
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                flex: 1,
              }}
            >
              {v.name}
            </span>
            <button
              onClick={(e) => handleRemoveVariant(e, v.id)}
              style={REMOVE_BTN}
              title="Remove variant"
            >
              &times;
            </button>
          </div>
        ))}
      </div>
      {/* Add variant */}
      <div style={{ padding: "6px 8px", display: "flex", gap: 4 }}>
        <input
          value={newVariantName}
          onChange={(e) => setNewVariantName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleAddVariant();
          }}
          placeholder="+ Add Variant"
          style={{
            ...INPUT_STYLE,
            flex: 1,
            minWidth: 0,
            fontSize: 11,
          }}
        />
        <button
          onClick={handleAddVariant}
          disabled={!newVariantName.trim()}
          style={{
            padding: "3px 8px",
            fontSize: 11,
            border: "none",
            borderRadius: 4,
            background: newVariantName.trim() ? "var(--dp-accent, #6366f1)" : "var(--dp-border, #e8e5f2)",
            color: newVariantName.trim() ? "#fff" : "var(--dp-text-muted, #a09db8)",
            cursor: newVariantName.trim() ? "pointer" : "default",
          }}
        >
          +
        </button>
      </div>
    </div>
  );

  // ── Render: Dimension section ──

  const renderDimensions = () => {
    if (!selectedVariant) return null;

    return (
      <div style={{ marginBottom: 12 }}>
        <div style={SECTION_HEADER}>Dimensions</div>
        {selectedVariant.dimensions.length === 0 && (
          <div style={{ fontSize: 12, color: "var(--dp-text-muted, #a09db8)", marginBottom: 6 }}>
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
                background: "var(--dp-bg, #f7f6fa)",
                borderRadius: 6,
                border: "1px solid var(--dp-border, #e8e5f2)",
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
                <span style={{ fontSize: 12, fontWeight: 600, color: "var(--dp-text, #1e1b4b)" }}>
                  {dim.name}
                </span>
                <span style={{ fontSize: 11, color: "var(--dp-text-muted, #a09db8)" }}>
                  ({dim.values.length} values)
                </span>
                <div style={{ flex: 1 }} />
                <button
                  onClick={() => handleRemoveDimension(dim.name)}
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
                    <span
                      key={val}
                      onClick={() => handleSetDefault(dim.name, val)}
                      style={isDefault ? VALUE_PILL_DEFAULT : VALUE_PILL}
                      title={
                        isDefault
                          ? `Default for ${dim.name}`
                          : `Click to set as default`
                      }
                    >
                      {val}
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleRemoveDimValue(dim.name, val);
                        }}
                        style={{
                          border: "none",
                          background: "none",
                          cursor: "pointer",
                          color: isDefault ? "rgba(255,255,255,0.7)" : "var(--dp-text-muted, #a09db8)",
                          fontSize: 12,
                          padding: 0,
                          lineHeight: 1,
                          marginLeft: 2,
                        }}
                        title="Remove value"
                      >
                        &times;
                      </button>
                    </span>
                  );
                })}
              </div>

              {/* Add value input */}
              <div style={{ display: "flex", gap: 4 }}>
                <input
                  value={newDimValue}
                  onChange={(e) => setNewDimValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleAddDimValue(dim.name);
                  }}
                  placeholder="+ value"
                  style={{
                    ...INPUT_STYLE,
                    width: 120,
                    fontSize: 11,
                  }}
                />
                <button
                  onClick={() => handleAddDimValue(dim.name)}
                  disabled={!newDimValue.trim()}
                  style={{
                    padding: "2px 8px",
                    fontSize: 11,
                    border: "1px solid var(--dp-border, #e8e5f2)",
                    borderRadius: "var(--dp-radius-sm, 6px)",
                    background: newDimValue.trim() ? "var(--dp-accent, #6366f1)" : "var(--dp-bg, #f7f6fa)",
                    color: newDimValue.trim() ? "#fff" : "var(--dp-text-muted, #a09db8)",
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
            value={newDimName}
            onChange={(e) => setNewDimName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAddDimension();
            }}
            placeholder='+ 添加维度 (e.g. "color", "size")'
            style={{ ...INPUT_STYLE, flex: 1, minWidth: 0, fontSize: 11 }}
          />
          <button
            onClick={handleAddDimension}
            disabled={!newDimName.trim()}
            style={{
              padding: "3px 8px",
              fontSize: 11,
              border: "none",
              borderRadius: 4,
              background: newDimName.trim() ? "var(--dp-accent, #6366f1)" : "var(--dp-border, #e8e5f2)",
              color: newDimName.trim() ? "#fff" : "var(--dp-text-muted, #a09db8)",
              cursor: newDimName.trim() ? "pointer" : "default",
              whiteSpace: "nowrap",
            }}
          >
            + 添加维度
          </button>
        </div>
      </div>
    );
  };

  // ── Render: Default combination ──

  const renderDefaultCombo = () => {
    if (!selectedVariant || Object.keys(selectedVariant.defaults).length === 0) return null;
    return (
      <div
        style={{
          fontSize: 11,
          color: "var(--dp-text-secondary, #6b6893)",
          padding: "6px 8px",
          background: "var(--dp-bg, #f7f6fa)",
          borderRadius: "var(--dp-radius-sm, 6px)",
          marginBottom: 12,
          border: "1px solid var(--dp-border, #e8e5f2)",
        }}
      >
        <span style={{ fontWeight: 500, color: "var(--dp-text, #1e1b4b)" }}>默认组合: </span>
        {defaultComboStr}
      </div>
    );
  };

  // ── Render: Branch tabs ──

  const renderBranchTabs = () => {
    if (!selectedVariant) return null;
    const branchesExist = branchKeys.length > 0;

    return (
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 4,
          padding: "4px 0",
          marginBottom: 8,
          borderBottom: "1px solid var(--dp-border, #e8e5f2)",
          paddingBottom: 8,
          alignItems: "center",
        }}
      >
        <button
          onClick={() => handleSelectBranch("base")}
          style={activeBranchKey === "base" ? BRANCH_TAB_ACTIVE : BRANCH_TAB}
        >
          base
        </button>
        {branchKeys.map((key) => (
          <button
            key={key}
            onClick={() => handleSelectBranch(key)}
            style={activeBranchKey === key ? BRANCH_TAB_ACTIVE : BRANCH_TAB}
          >
            {key}
          </button>
        ))}
        {!branchesExist && (
          <span style={{ fontSize: 11, color: "var(--dp-text-muted, #a09db8)", marginLeft: 4 }}>
            (add dimension values above)
          </span>
        )}
      </div>
    );
  };

  // ── Render: Add property dropdown ──

  const renderAddProperty = () => {
    const filtered = ALL_PROPERTY_NAMES.filter(
      (n) => n.toLowerCase().includes(addPropSearch.toLowerCase()),
    );

    return (
      <div style={{ position: "relative", margin: "4px 8px" }}>
        <button
          onClick={() => setShowAddProp((v) => !v)}
          style={DASHED_BTN}
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
                onClick={() => {
                  handleSinglePropChange(name, "");
                  setShowAddProp(false);
                  setAddPropSearch("");
                }}
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

  // ── Render: Cards-based property editor ──

  const renderCards = () => {
    if (!selectedVariant) return null;

    return (
      <div>
        {/* Visual editing cards */}
        <SizeCard
          properties={currentProps}
          onChange={handleCardPropsChange}
        />
        <BackgroundCard
          properties={currentProps}
          tokens={tokens}
          onChange={handleCardPropsChange}
        />
        <BorderCard
          properties={currentProps}
          tokens={tokens}
          onChange={handleCardPropsChange}
        />
        <RadiusCard
          properties={currentProps}
          onChange={handleCardPropsChange}
        />
        <SpacingCard
          properties={currentProps}
          onChange={handleCardPropsChange}
        />

        {/* Layout group for remaining layout properties */}
        <CollapsibleGroup title="布局 Layout" summary={layoutSummary} defaultOpen={false}>
          {layoutPropNames.map((name) => {
            const value = getPropValue(currentProps, name);
            const label = PROPERTY_LABELS[name] || name;
            const isInherited = isBranchMode && !branch?.properties.some((p) => p.name === name);

            if (isInherited) {
              const baseValue = getPropValue(selectedVariant.baseProperties, name);
              return (
                <PropertyRow key={name} label={label} inherited>
                  <div
                    style={{
                      ...INPUT_STYLE,
                      background: "var(--dp-bg, #f7f6fa)",
                      color: "var(--dp-text-muted, #a09db8)",
                      cursor: "not-allowed",
                      display: "flex",
                      alignItems: "center",
                      fontSize: 11,
                    }}
                  >
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
                  onChange={(v) => handleSinglePropChange(name, v as string)}
                />
              );
            } else if (name === "gap") {
              input = (
                <ScrubInput
                  value={parseFloat(value) || 0}
                  unit="px"
                  min={0}
                  max={200}
                  onChange={(v) => handleSinglePropChange(name, `${v}px`)}
                />
              );
            } else if (isEnumProperty(name)) {
              input = (
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
              );
            } else {
              input = (
                <input
                  value={value}
                  onChange={(e) => handleSinglePropChange(name, e.target.value)}
                  style={INPUT_STYLE}
                  placeholder="value"
                />
              );
            }

            return (
              <PropertyRow
                key={name}
                label={label}
                showOverrideDot={isBranchMode}
                onRemove={() => handleRemoveProperty(name)}
              >
                {input}
              </PropertyRow>
            );
          })}
        </CollapsibleGroup>

        {/* Typography group */}
        <CollapsibleGroup title="文字 Typography" summary={typographySummary} defaultOpen={false}>
          {typographyPropNames.map((name) => {
            const value = getPropValue(currentProps, name);
            const label = PROPERTY_LABELS[name] || name;
            const isInherited = isBranchMode && !branch?.properties.some((p) => p.name === name);

            if (isInherited) {
              const baseValue = getPropValue(selectedVariant.baseProperties, name);
              return (
                <PropertyRow key={name} label={label} inherited>
                  <div
                    style={{
                      ...INPUT_STYLE,
                      background: "var(--dp-bg, #f7f6fa)",
                      color: "var(--dp-text-muted, #a09db8)",
                      cursor: "not-allowed",
                      display: "flex",
                      alignItems: "center",
                      fontSize: 11,
                    }}
                  >
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
                  onChange={(v) => handleSinglePropChange(name, unit ? `${v}${unit}` : `${v}`)}
                />
              );
            } else if (isEnumProperty(name)) {
              input = (
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
              );
            } else {
              input = (
                <input
                  value={value}
                  onChange={(e) => handleSinglePropChange(name, e.target.value)}
                  style={INPUT_STYLE}
                  placeholder="value"
                />
              );
            }

            return (
              <PropertyRow
                key={name}
                label={label}
                showOverrideDot={isBranchMode}
                onRemove={() => handleRemoveProperty(name)}
              >
                {input}
              </PropertyRow>
            );
          })}
        </CollapsibleGroup>

        {renderAddProperty()}
      </div>
    );
  };

  // ── Render: Preview panel ──

  const renderPreview = () => {
    if (!selectedVariant) return null;
    return (
      <div
        style={{
          width: 260,
          flexShrink: 0,
          borderLeft: "1px solid var(--dp-border, #e8e5f2)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          background: "var(--dp-surface, #fff)",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "8px 12px",
            fontSize: 10,
            fontWeight: 600,
            color: "var(--dp-text-secondary, #6b6893)",
            textTransform: "uppercase",
            letterSpacing: "0.5px",
            borderBottom: "1px solid var(--dp-border, #e8e5f2)",
            flexShrink: 0,
          }}
        >
          Preview
        </div>

        {/* Dimension combination selectors */}
        {selectedVariant.dimensions.map((dim) => (
          <div
            key={dim.name}
            style={{
              padding: "4px 8px",
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <span
              style={{
                fontSize: 11,
                fontWeight: 500,
                color: "var(--dp-text-secondary, #6b6893)",
                minWidth: 40,
              }}
            >
              {dim.name}
            </span>
            <select
              value={previewSelections[dim.name] || ""}
              onChange={(e) =>
                setPreviewSelections((prev) => ({
                  ...prev,
                  [dim.name]: e.target.value,
                }))
              }
              style={{
                flex: 1,
                padding: "2px 4px",
                fontSize: 11,
                fontFamily: "var(--dp-font-mono, monospace)",
                border: "1px solid var(--dp-border, #e8e5f2)",
                borderRadius: "var(--dp-radius-sm, 6px)",
                outline: "none",
                background: "var(--dp-surface, #fff)",
                color: "var(--dp-text, #1e1b4b)",
                cursor: "pointer",
              }}
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
  };

  // ── Main render ──

  return (
    <div
      style={{
        display: "flex",
        height: "100%",
        fontSize: 13,
        color: "var(--dp-text, #1e1b4b)",
        position: "relative",
      }}
    >
      {renderSidebar()}

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
                  color: "var(--dp-text, #1e1b4b)",
                  marginBottom: 8,
                }}
              >
                {selectedVariant.name}
              </div>

              {/* Default combination */}
              {renderDefaultCombo()}

              {/* Dimensions */}
              {renderDimensions()}

              {/* Branch tabs */}
              {renderBranchTabs()}

              {/* Cards-based property editor */}
              {renderCards()}
            </div>
          ) : (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                color: "var(--dp-text-muted, #a09db8)",
                fontSize: 13,
              }}
            >
              Select a variant to edit
            </div>
          )}
        </div>
        {selectedVariant && renderPreview()}
      </div>
    </div>
  );
}
