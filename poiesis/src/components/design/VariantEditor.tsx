// components/design/VariantEditor.tsx
// Variant editor with dimensions tree, base editor, and branch editor.

import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils";
import type { VariantDimension } from "../../core/types/design";
import type { StyleProperty, StyleValueSegment } from "../../core/types/style";

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

const ITEM_BASE: React.CSSProperties = {
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

const ITEM_ACTIVE: React.CSSProperties = {
  ...ITEM_BASE,
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


const PROP_ROW: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "3px 0",
};

const PROP_LABEL: React.CSSProperties = {
  fontSize: 11,
  color: "#374151",
  width: 100,
  flexShrink: 0,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const INHERITED_LABEL: React.CSSProperties = {
  ...PROP_LABEL,
  color: "#94a3b8",
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

const DASHED_BTN: React.CSSProperties = {
  border: "1px dashed #cbd5e1",
  borderRadius: 4,
  background: "transparent",
  color: "#64748b",
  fontSize: 10,
  padding: "4px 10px",
  cursor: "pointer",
  width: "100%",
};

const VALUE_PILL: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
  padding: "2px 8px",
  fontSize: 11,
  borderRadius: 12,
  border: "1px solid #e2e8f0",
  background: "#f8fafc",
  color: "#374151",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const VALUE_PILL_DEFAULT: React.CSSProperties = {
  ...VALUE_PILL,
  background: "#6366f1",
  color: "#fff",
  borderColor: "#6366f1",
};

const BRANCH_TAB: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 11,
  cursor: "pointer",
  border: "none",
  background: "none",
  color: "#64748b",
  borderRadius: 4,
};

const BRANCH_TAB_ACTIVE: React.CSSProperties = {
  ...BRANCH_TAB,
  background: "#eef2ff",
  color: "#4f46e5",
  fontWeight: 600,
};

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "12px 0 6px",
};

// ── Component ──

export function VariantEditor() {
  // ── Store ──
  const variants = usePoiesisStore((s) => s.activeDesign.variants);
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
  const [addPropName, setAddPropName] = useState("");

  const addPropRef = useRef<HTMLDivElement>(null);

  // ── Derived data ──

  const selectedVariant = useMemo(
    () => variants.find((v) => v.id === selectedVariantId) ?? null,
    [variants, selectedVariantId],
  );

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

  // Default combination display string
  const defaultComboStr = useMemo(() => {
    if (!selectedVariant) return "";
    return Object.entries(selectedVariant.defaults)
      .map(([k, v]) => `${k}=${v}`)
      .join(", ");
  }, [selectedVariant]);

  // ── Close add-property popup on outside click ──

  useEffect(() => {
    if (!showAddProp) return;
    const handler = (e: MouseEvent) => {
      if (addPropRef.current && !addPropRef.current.contains(e.target as Node)) {
        setShowAddProp(false);
        setAddPropName("");
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [showAddProp]);

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

  const updatePropsList = useCallback(
    (props: StyleProperty[], propName: string, value: string): StyleProperty[] => {
      const idx = props.findIndex((p) => p.name === propName);
      if (idx >= 0) {
        const next = [...props];
        next[idx] = { ...next[idx], segments: stringToSegments(value) };
        return next;
      }
      return [
        ...props,
        { id: uid("prop"), name: propName, segments: stringToSegments(value) },
      ];
    },
    [],
  );

  const removePropFromList = useCallback(
    (props: StyleProperty[], propName: string): StyleProperty[] =>
      props.filter((p) => p.name !== propName),
    [],
  );

  const handlePropertyChange = useCallback(
    (propName: string, value: string) => {
      if (!selectedVariant) return;

      if (activeBranchKey === "base") {
        updateVariant(selectedVariant.id, {
          baseProperties: updatePropsList(selectedVariant.baseProperties, propName, value),
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
              ? { ...b, properties: updatePropsList(b.properties, propName, value) }
              : b,
          ),
        });
      }
    },
    [selectedVariant, activeBranchKey, updateVariant, updatePropsList],
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

  const handleAddPropertyNameSubmit = useCallback(() => {
    const name = addPropName.trim();
    if (!name) return;
    handlePropertyChange(name, "");
    setShowAddProp(false);
    setAddPropName("");
  }, [addPropName, handlePropertyChange]);

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
            background: newVariantName.trim() ? "#6366f1" : "#e2e8f0",
            color: newVariantName.trim() ? "#fff" : "#94a3b8",
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
          <div style={{ fontSize: 11, color: "#94a3b8", marginBottom: 6 }}>
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
                background: "#fafafa",
                borderRadius: 6,
                border: "1px solid #f0f0f0",
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
                <span style={{ fontSize: 12, fontWeight: 600, color: "#374151" }}>
                  {dim.name}
                </span>
                <span style={{ fontSize: 10, color: "#94a3b8" }}>
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
                          color: isDefault ? "rgba(255,255,255,0.7)" : "#94a3b8",
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
                    fontSize: 10,
                  }}
                />
                <button
                  onClick={() => handleAddDimValue(dim.name)}
                  disabled={!newDimValue.trim()}
                  style={{
                    padding: "2px 8px",
                    fontSize: 10,
                    border: "1px solid #e2e8f0",
                    borderRadius: 4,
                    background: newDimValue.trim() ? "#6366f1" : "#f8fafc",
                    color: newDimValue.trim() ? "#fff" : "#94a3b8",
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
            style={{ ...INPUT_STYLE, flex: 1, minWidth: 0, fontSize: 10 }}
          />
          <button
            onClick={handleAddDimension}
            disabled={!newDimName.trim()}
            style={{
              padding: "3px 8px",
              fontSize: 11,
              border: "none",
              borderRadius: 4,
              background: newDimName.trim() ? "#6366f1" : "#e2e8f0",
              color: newDimName.trim() ? "#fff" : "#94a3b8",
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
          color: "#64748b",
          padding: "6px 8px",
          background: "#f8fafc",
          borderRadius: 6,
          marginBottom: 12,
          border: "1px solid #e2e8f0",
        }}
      >
        <span style={{ fontWeight: 500, color: "#374151" }}>默认组合: </span>
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
          borderBottom: "1px solid #e2e8f0",
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
          <span style={{ fontSize: 10, color: "#94a3b8", marginLeft: 4 }}>
            (add dimension values above)
          </span>
        )}
      </div>
    );
  };

  // ── Render: Property rows ──

  const renderPropertyRow = (
    propName: string,
    prop: StyleProperty | undefined,
    inherited: boolean,
  ) => {
    if (!selectedVariant) return null;
    const displayValue = prop ? segmentsToString(prop.segments) : "";

    return (
      <div key={propName} style={PROP_ROW}>
        {/* Override indicator dot */}
        {!inherited && activeBranchKey !== "base" && (
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: "#6366f1",
              flexShrink: 0,
              display: "inline-block",
            }}
          />
        )}
        {inherited && <span style={{ width: 6, flexShrink: 0, display: "inline-block" }} />}

        {/* Property name */}
        <span style={inherited ? INHERITED_LABEL : PROP_LABEL} title={propName}>
          {propName}
        </span>

        {/* Value */}
        {!inherited && (
          <input
            value={displayValue}
            onChange={(e) => handlePropertyChange(propName, e.target.value)}
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
              fontSize: 10,
            }}
          >
            ← base: {displayValue || "—"}
          </div>
        )}

        {/* Remove */}
        {!inherited && (
          <button
            onClick={() => handleRemoveProperty(propName)}
            style={REMOVE_BTN}
            title="Remove property"
          >
            &times;
          </button>
        )}
      </div>
    );
  };

  // ── Render: Property editor ──

  const renderPropertyEditor = () => {
    if (!selectedVariant) return null;

    if (activeBranchKey === "base") {
      // Base mode: show only base properties
      const baseProps = selectedVariant.baseProperties;
      return (
        <div>
          {baseProps.length === 0 && (
            <div style={{ fontSize: 11, color: "#94a3b8", marginBottom: 8 }}>
              No base properties yet.
            </div>
          )}
          {baseProps.map((prop) => renderPropertyRow(prop.name, prop, false))}
          {renderAddPropertyButton()}
        </div>
      );
    }

    // Branch mode: show inherited base properties + branch overrides
    const parsed = parseBranchKey(activeBranchKey);
    if (!parsed) return null;
    const baseProps = selectedVariant.baseProperties;
    const branch = selectedVariant.branches.find(
      (b) => b.dimension === parsed.dim && b.value === parsed.val,
    );
    const branchProps = branch?.properties ?? [];

    // Collect all property names from base and branch
    const allPropNames = new Set<string>();
    for (const bp of baseProps) allPropNames.add(bp.name);
    for (const bp of branchProps) allPropNames.add(bp.name);

    const propNames = Array.from(allPropNames);

    return (
      <div>
        {propNames.length === 0 && (
          <div style={{ fontSize: 11, color: "#94a3b8", marginBottom: 8 }}>
            No properties for this branch. Add one below, or properties inherited from base will
            appear here.
          </div>
        )}
        {propNames.map((name) => {
          const baseProp = baseProps.find((p) => p.name === name);
          const branchProp = branchProps.find((p) => p.name === name);
          const baseVal = baseProp ? segmentsToString(baseProp.segments) : "";
          const branchVal = branchProp ? segmentsToString(branchProp.segments) : "";
          const isOverridden = branchProp !== undefined && branchVal !== baseVal;
          const isNew = branchProp !== undefined && !baseProp;
          const editable = isOverridden || isNew;
          return renderPropertyRow(name, editable ? branchProp : baseProp, !editable);
        })}
        {renderAddPropertyButton()}
      </div>
    );
  };

  // ── Render: Add property button with inline input ──

  const renderAddPropertyButton = () => {
    return (
      <div style={{ position: "relative", marginTop: 6 }}>
        <button onClick={() => setShowAddProp((v) => !v)} style={DASHED_BTN}>
          + 添加属性
        </button>
        {showAddProp && (
          <div
            ref={addPropRef}
            style={{
              marginTop: 4,
              display: "flex",
              gap: 4,
              alignItems: "center",
            }}
          >
            <input
              autoFocus
              value={addPropName}
              onChange={(e) => setAddPropName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleAddPropertyNameSubmit();
                if (e.key === "Escape") {
                  setShowAddProp(false);
                  setAddPropName("");
                }
              }}
              placeholder="Property name"
              style={{
                ...INPUT_STYLE,
                flex: 1,
                minWidth: 0,
                fontSize: 11,
                padding: "4px 6px",
              }}
            />
            <button
              onClick={handleAddPropertyNameSubmit}
              disabled={!addPropName.trim()}
              style={{
                padding: "4px 8px",
                fontSize: 11,
                border: "none",
                borderRadius: 4,
                background: addPropName.trim() ? "#6366f1" : "#e2e8f0",
                color: addPropName.trim() ? "#fff" : "#94a3b8",
                cursor: addPropName.trim() ? "pointer" : "default",
              }}
            >
              Add
            </button>
          </div>
        )}
      </div>
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
        }}
      >
        {selectedVariant ? (
          <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
            {/* Variant name header */}
            <div
              style={{
                fontSize: 14,
                fontWeight: 600,
                color: "#1e293b",
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

            {/* Property editor */}
            {renderPropertyEditor()}
          </div>
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
            Select a variant to edit
          </div>
        )}
      </div>
    </div>
  );
}
