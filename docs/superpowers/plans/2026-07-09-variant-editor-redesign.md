# Variant Editor Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade VariantEditor's property editing from raw text inputs to ClassEditor-style visual cards (Size/Background/Border/Radius/Spacing) with `/` token binding and a live preview panel.

**Architecture:** Keep existing sidebar + dimension management + branch tabs. Replace the property editing area with cards and add a right-side ClassPreview. Cards operate on a merged `currentProps` (base + active branch overrides); a diff helper determines which edits become branch overrides. The `/` token picker is a shared popover component.

**Tech Stack:** React + TypeScript, Zustand store, existing ClassPreview/card components

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `poiesis/src/components/design/VariantEditor.tsx` | **Modify** | Main refactor: cards, preview, `/` token binding |
| `poiesis/src/components/design/TokenSlashPicker.tsx` | **Create** | Reusable `/` token popover for property inputs |
| `poiesis/src/components/design/ClassPreview.tsx` | **No change** | Reused as-is for live preview |

Cards `SizeCard`, `BackgroundCard`, `BorderCard`, `RadiusCard`, `SpacingCard` — reused as-is, no changes.

---

### Task 1: Add style merge helper for Variant preview

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx` (add helper before component)

- [ ] **Step 1: Add `getCombinedStyle` helper function**

Insert after the existing `parseBranchKey` helper (line 26) and before the styles section:

```typescript
/** Merge base properties with active branch overrides into a single flat array.
 *  Branch properties override base properties with the same name. */
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

/** Compute branch override properties: properties in merged that differ from base. */
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
```

- [ ] **Step 2: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -20`
Expected: No new errors from VariantEditor.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: add getCombinedProps and computeBranchOverrides helpers for VariantEditor"
```

---

### Task 2: Compute currentProps and wire card onChange for base + branch modes

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Add `currentProps` derived state**

Insert after the existing `branchKeys` useMemo (line 214):

```typescript
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
```

- [ ] **Step 2: Add `handleCardPropsChange` callback**

Insert after `handlePropertyChange` (line 409):

```typescript
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
```

- [ ] **Step 3: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -20`

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: add currentProps and handleCardPropsChange for card-based variant editing"
```

---

### Task 3: Replace raw property rows with card components

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Add card imports**

Replace/add imports at top of file (after existing imports, line 7):

```typescript
import { SizeCard } from "./cards/SizeCard";
import { BackgroundCard } from "./cards/BackgroundCard";
import { BorderCard } from "./cards/BorderCard";
import { RadiusCard } from "./cards/RadiusCard";
import { SpacingCard } from "./cards/SpacingCard";
import { CollapsibleGroup } from "./CollapsibleGroup";
import { PropertyRow } from "./PropertyRow";
import { ScrubInput } from "./ScrubInput";
import { IconButtonGroup } from "./IconButtonGroup";
```

- [ ] **Step 2: Read tokens from store**

Add tokens selector in the store destructuring section (after line 184):

```typescript
const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
```

- [ ] **Step 3: Add card rendering helper constants**

Add after the existing `SECTION_HEADER` style (line 176):

```typescript
// ── Property metadata (mirrors ClassEditor) ──

const PROPERTY_LABELS: Record<string, string> = {
  display: "Display", position: "定位", width: "宽度", height: "高度",
  flexDirection: "Flex 方向", justifyContent: "主轴对齐", alignItems: "交叉轴对齐",
  alignSelf: "自身对齐", padding: "内边距", paddingTop: "内边距-上",
  paddingRight: "内边距-右", paddingBottom: "内边距-下", paddingLeft: "内边距-左",
  margin: "外边距", marginTop: "外边距-上", marginRight: "外边距-右",
  marginBottom: "外边距-下", marginLeft: "外边距-左", gap: "间距",
  overflow: "溢出", overflowX: "溢出-X", overflowY: "溢出-Y", flexWrap: "换行",
  background: "背景色", border: "边框", borderColor: "边框颜色",
  borderRadius: "圆角", boxShadow: "阴影", opacity: "不透明度",
  cursor: "光标", transition: "过渡", transform: "变换", color: "文字颜色",
  fontFamily: "字体", fontSize: "字号", fontWeight: "字重",
  lineHeight: "行高", textAlign: "对齐", textDecoration: "装饰线",
  textTransform: "大小写", letterSpacing: "字间距",
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

const ALL_PROPERTY_NAMES = [
  "display", "position", "width", "height",
  "flexDirection", "justifyContent", "alignItems", "alignSelf",
  "padding", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
  "margin", "marginTop", "marginRight", "marginBottom", "marginLeft",
  "gap", "overflow", "overflowX", "overflowY", "flexWrap",
  "background", "border", "borderColor", "borderRadius",
  "boxShadow", "opacity", "cursor", "transition", "transform",
  "color", "fontFamily", "fontSize", "fontWeight",
  "lineHeight", "textAlign", "textDecoration", "textTransform", "letterSpacing",
];

const layoutPropNames = ["display", "position", "flexDirection", "justifyContent", "alignItems", "alignSelf", "gap", "overflow", "overflowX", "overflowY", "flexWrap"];
const typographyPropNames = ["color", "fontFamily", "fontSize", "fontWeight", "lineHeight", "textAlign", "textDecoration", "textTransform", "letterSpacing"];
```

- [ ] **Step 4: Add `getPropValue`, `isEnumProperty`, `handleSinglePropChange` helpers**

In the helpers section after `stringToSegments`:

```typescript
function getPropValue(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
  if (!prop || prop.segments.length === 0) return "";
  const seg = prop.segments[0];
  return seg.type === "text" ? seg.value : "";
}

function isEnumProperty(name: string): boolean {
  return name in ENUM_OPTIONS;
}
```

- [ ] **Step 5: Add `handleSinglePropChange` callback (for CollapsibleGroup rows)**

After `handleCardPropsChange`:

```typescript
/** Handle a single property change from CollapsibleGroup rows.
 *  Mirrors ClassEditor's single-prop pattern. */
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
```

- [ ] **Step 6a: Add component-level state for add-property and summaries**

Add these state declarations in the component's local state section (near other `useState` calls, after line 193):

```typescript
const [showAddProp, setShowAddProp] = useState(false);
const [addPropSearch, setAddPropSearch] = useState("");
const addPropRef = useRef<HTMLDivElement>(null);
```

Add these useMemo declarations after `currentProps` (from Task 2):

```typescript
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
```

- [ ] **Step 6b: Replace `renderPropertyEditor` with card-based rendering**

Replace the existing `renderPropertyEditor` function (line 810-866) AND `renderAddPropertyButton` (line 870-925) with:

```typescript
const renderAddProperty = () => {
  const filtered = ALL_PROPERTY_NAMES.filter(
    (n) => n.toLowerCase().includes(addPropSearch.toLowerCase()),
  );

  return (
    <div style={{ position: "relative", margin: "4px 8px" }}>
      <button
        onClick={() => setShowAddProp((v) => !v)}
        style={{
          border: "1px dashed var(--dp-border, #e8e5f2)",
          borderRadius: "var(--dp-radius-sm, 6px)",
          background: "transparent",
          color: "var(--dp-text-muted, #a09db8)",
          fontSize: 11,
          padding: "6px 10px",
          cursor: "pointer",
          width: "100%",
          fontFamily: "inherit",
          transition: "color 150ms ease, border-color 150ms ease",
        }}
      >
        + Add property
      </button>
      {showAddProp && (
        <div
          ref={addPropRef}
          style={{
            position: "absolute", top: "100%", left: 0, right: 0, marginTop: 2,
            background: "#fff", border: "1px solid #e2e8f0", borderRadius: 6,
            boxShadow: "0 4px 12px rgba(0,0,0,0.1)", zIndex: 100, maxHeight: 200,
            overflow: "auto",
          }}
        >
          <input
            autoFocus
            value={addPropSearch}
            onChange={(e) => setAddPropSearch(e.target.value)}
            placeholder="Search properties..."
            style={{ ...INPUT_STYLE, margin: 4, width: "calc(100% - 8px)", padding: "4px 6px", fontSize: 11 }}
          />
          {filtered.map((name) => (
            <button
              key={name}
              onClick={() => { handleSinglePropChange(name, ""); setShowAddProp(false); setAddPropSearch(""); }}
              style={{
                display: "block", width: "100%", padding: "4px 10px", fontSize: 11,
                border: "none", background: "transparent", color: "#374151",
                cursor: "pointer", textAlign: "left",
              }}
              onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "#f1f5f9"; }}
              onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
            >
              <span style={{ color: "#64748b", marginRight: 4 }}>+</span>
              {PROPERTY_LABELS[name] || name}
            </button>
          ))}
          {filtered.length === 0 && (
            <div style={{ padding: 8, fontSize: 10, color: "#94a3b8", textAlign: "center" }}>
              No matching properties
            </div>
          )}
        </div>
      )}
    </div>
  );
};

const renderCards = () => {
  if (!selectedVariant) return null;

  // In branch mode, check if the branch exists to determine inheritance
  const isBranchMode = activeBranchKey !== "base";
  const parsed = isBranchMode ? parseBranchKey(activeBranchKey) : null;
  const branch = parsed
    ? selectedVariant.branches.find((b) => b.dimension === parsed.dim && b.value === parsed.val) ?? null
    : null;

  // For cards, we pass currentProps (merged) directly. Cards always edit what they see.
  // In base mode, edits go to baseProperties. In branch mode, diffs determine overrides.

  return (
    <div style={{ overflow: "auto", padding: "8px 0", background: "var(--dp-bg, #f7f6fa)" }}>
      {/* Cards */}
      <SizeCard properties={currentProps} onChange={handleCardPropsChange} />
      <BackgroundCard properties={currentProps} tokens={tokens} onChange={handleCardPropsChange} />
      <BorderCard properties={currentProps} tokens={tokens} onChange={handleCardPropsChange} />
      <RadiusCard properties={currentProps} onChange={handleCardPropsChange} />
      <SpacingCard properties={currentProps} onChange={handleCardPropsChange} />

      {/* Layout group */}
      <CollapsibleGroup title="布局 Layout" summary={layoutSummary} defaultOpen={false}>
        {layoutPropNames.map((name) => {
          const value = getPropValue(currentProps, name);
          const label = PROPERTY_LABELS[name] || name;
          const isInherited = isBranchMode && !branch?.properties.some((p) => p.name === name);
          if (name === "flexDirection") {
            return (
              <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
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
              <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
                <ScrubInput value={parseFloat(value) || 0} unit="px" min={0} max={200}
                  onChange={(v) => handleSinglePropChange(name, `${v}px`)} />
              </PropertyRow>
            );
          }
          if (isEnumProperty(name)) {
            return (
              <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
                <select value={value} onChange={(e) => handleSinglePropChange(name, e.target.value)}
                  style={SELECT_STYLE}>
                  <option value="">—</option>
                  {ENUM_OPTIONS[name]?.map((opt) => (
                    <option key={opt} value={opt}>{opt}</option>
                  )) ?? null}
                </select>
              </PropertyRow>
            );
          }
          return (
            <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
              <input value={value} onChange={(e) => handleSinglePropChange(name, e.target.value)}
                style={INPUT_STYLE} placeholder="value" />
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
          if (["fontSize", "lineHeight", "letterSpacing"].includes(name)) {
            const unit = name === "fontSize" ? "px" : (name === "letterSpacing" ? "px" : "");
            return (
              <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
                <ScrubInput value={parseFloat(value) || 0} unit={unit} min={0} max={999}
                  onChange={(v) => handleSinglePropChange(name, unit ? `${v}${unit}` : `${v}`)} />
              </PropertyRow>
            );
          }
          if (isEnumProperty(name)) {
            return (
              <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
                <select value={value} onChange={(e) => handleSinglePropChange(name, e.target.value)}
                  style={SELECT_STYLE}>
                  <option value="">—</option>
                  {ENUM_OPTIONS[name]?.map((opt) => (
                    <option key={opt} value={opt}>{opt}</option>
                  )) ?? null}
                </select>
              </PropertyRow>
            );
          }
          return (
            <PropertyRow key={name} label={label} inherited={isInherited} showOverrideDot={!isInherited && isBranchMode}>
              <input value={value} onChange={(e) => handleSinglePropChange(name, e.target.value)}
                style={INPUT_STYLE} placeholder="value" />
            </PropertyRow>
          );
        })}
      </CollapsibleGroup>

      {renderAddProperty()}
    </div>
  );
};
```

**Important:** When adding the `renderCards` function, also add the `useOutsideClick` hook from ClassEditor to VariantEditor (a copy, since it's not extracted). Add it before the component function:

```typescript
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
```

And add a `SELECT_STYLE` constant (copy from ClassEditor line 224-236):

```typescript
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
```

- [ ] **Step 7: Update main render to use cards instead of property rows**

In the main render section (line 929-992), replace:

```tsx
{/* Property editor */}
{renderPropertyEditor()}
```

With:

```tsx
{/* Property cards */}
{renderCards()}
```

- [ ] **Step 8: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -30`
Expected: No new errors from VariantEditor.

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: replace raw property rows with ClassEditor cards in VariantEditor"
```

---

### Task 4: Add live preview panel with combination selectors

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Add preview state and preview style merging**

Add local state for preview dimension selections:

```typescript
// Preview dimension selections — initialized from variant defaults
const [previewSelections, setPreviewSelections] = useState<Record<string, string>>({});
```

Add a useEffect to initialize preview selections when variant changes:

```typescript
// Sync preview selections to variant defaults on variant change
useEffect(() => {
  if (selectedVariant) {
    setPreviewSelections({ ...selectedVariant.defaults });
  }
}, [selectedVariant?.id]); // eslint-disable-line react-hooks/exhaustive-deps
```

- [ ] **Step 2: Compute preview props**

Add after the preview state:

```typescript
// Preview properties: merge base + selected dimension branches
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
```

- [ ] **Step 3: Add ClassPreview import**

```typescript
import { ClassPreview } from "./ClassPreview";
```

- [ ] **Step 4: Add renderPreview function**

```typescript
const renderPreview = () => {
  if (!selectedVariant) return null;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        width: 260,
        flexShrink: 0,
        borderLeft: "1px solid var(--dp-border, #e8e5f2)",
        background: "var(--dp-surface, #fff)",
      }}
    >
      {/* Header with dimension selectors */}
      <div
        style={{
          padding: "8px 12px",
          fontSize: 10,
          fontWeight: 600,
          color: "var(--dp-text-secondary, #6b6893)",
          textTransform: "uppercase",
          letterSpacing: "0.5px",
          borderBottom: "1px solid var(--dp-border, #e8e5f2)",
          background: "var(--dp-bg, #f7f6fa)",
          display: "flex",
          flexDirection: "column",
          gap: 4,
        }}
      >
        Preview
        {selectedVariant.dimensions.map((dim) => (
          <div key={dim.name} style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <span style={{ fontSize: 10, color: "var(--dp-text-muted, #a09db8)", textTransform: "none", fontWeight: 400, minWidth: 40 }}>
              {dim.name}:
            </span>
            <select
              value={previewSelections[dim.name] ?? ""}
              onChange={(e) =>
                setPreviewSelections((prev) => ({ ...prev, [dim.name]: e.target.value }))
              }
              style={{
                flex: 1,
                padding: "2px 4px",
                fontSize: 10,
                fontFamily: "var(--dp-font-mono, monospace)",
                border: "1px solid var(--dp-border, #e8e5f2)",
                borderRadius: "var(--dp-radius-sm, 6px)",
                outline: "none",
                background: "var(--dp-surface, #fff)",
                color: "var(--dp-text, #1e1b4b)",
                cursor: "pointer",
                textTransform: "none",
                letterSpacing: "normal",
                fontWeight: 500,
              }}
            >
              {dim.values.map((v) => (
                <option key={v} value={v}>{v}</option>
              ))}
            </select>
          </div>
        ))}
      </div>

      {/* ClassPreview (reuses preview rendering without pseudo tabs) */}
      <ClassPreview
        baseProperties={previewProps}
        pseudoStates={[]}
        className={selectedVariant.name}
        tokens={tokens}
      />
    </div>
  );
};
```

- [ ] **Step 5: Update main render layout**

Change the main render's content area (line 941-991) from single-column to two-column (editor + preview):

Replace the existing center content code with:

```tsx
<div
  style={{
    flex: 1,
    display: "flex",
    overflow: "hidden",
    minHeight: 0,
  }}
>
  {/* Property editor (cards) */}
  <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
    {selectedVariant ? (
      <div style={{ flex: 1, overflow: "auto" }}>
        {/* Variant name header */}
        <div
          style={{
            fontSize: 14,
            fontWeight: 600,
            color: "var(--dp-text, #1e1b4b)",
            padding: "8px 12px 4px",
          }}
        >
          {selectedVariant.name}
        </div>

        {/* Default combination */}
        {renderDefaultCombo()}

        {/* Dimensions */}
        <div style={{ padding: "0 12px" }}>{renderDimensions()}</div>

        {/* Branch tabs */}
        <div style={{ padding: "0 12px" }}>{renderBranchTabs()}</div>

        {/* Property cards */}
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

  {/* Preview panel */}
  {selectedVariant && renderPreview()}
</div>
```

- [ ] **Step 6: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -30`
Expected: No new errors.

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: add live preview panel with dimension combination selectors to VariantEditor"
```

---

### Task 5: Add `/` token slash picker

**Files:**
- Create: `poiesis/src/components/design/TokenSlashPicker.tsx`

- [ ] **Step 1: Create TokenSlashPicker component**

```typescript
// components/design/TokenSlashPicker.tsx
// Slash-command token picker: typing "/" in a property input opens this popover.

import React, { useRef, useEffect, useCallback } from "react";
import type { DesignToken } from "../../core/types/design";

interface TokenSlashPickerProps {
  tokens: DesignToken[];
  filter: string; // text after "/"
  onSelect: (tokenName: string) => void;
  onClose: () => void;
  position: { top: number; left: number };
}

const CATEGORY_LABELS: Record<string, string> = {
  colors: "Colors",
  spacing: "Spacing",
  typography: "Typography",
  radii: "Radii",
  shadows: "Shadows",
  durations: "Durations",
};

export function TokenSlashPicker({
  tokens,
  filter,
  onSelect,
  onClose,
  position,
}: TokenSlashPickerProps) {
  const ref = useRef<HTMLDivElement>(null);

  // Group and filter tokens
  const grouped = React.useMemo(() => {
    const query = filter.toLowerCase();
    const result: Record<string, DesignToken[]> = {};
    for (const t of tokens) {
      if (query && !t.name.toLowerCase().includes(query)) continue;
      const cat = t.category ?? "other";
      if (!result[cat]) result[cat] = [];
      result[cat].push(t);
    }
    return result;
  }, [tokens, filter]);

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  const categories = Object.keys(grouped);
  if (categories.length === 0) return null;

  return (
    <div
      ref={ref}
      style={{
        position: "fixed",
        top: position.top,
        left: position.left,
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
      {categories.map((cat) => (
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
            {CATEGORY_LABELS[cat] || cat}
          </div>
          {grouped[cat].map((t) => (
            <button
              key={t.id}
              onClick={() => onSelect(t.name)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                width: "100%",
                padding: "4px 8px",
                fontSize: 11,
                border: "none",
                background: "transparent",
                color: "var(--dp-text-secondary, #6b6893)",
                cursor: "pointer",
                textAlign: "left",
                fontFamily: "var(--dp-font-mono, monospace)",
                borderRadius: 4,
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
                  fontSize: 9,
                  color: "#94a3b8",
                }}
              >
                {t.value}
              </span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/TokenSlashPicker.tsx
git commit -m "feat: add TokenSlashPicker — slash-command token selector for property inputs"
```

---

### Task 6: Wire `/` token picker into VariantEditor property inputs

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Add token picker state**

Import TokenSlashPicker:

```typescript
import { TokenSlashPicker } from "./TokenSlashPicker";
```

Add local state for the token picker (after other useState declarations):

```typescript
const [slashPicker, setSlashPicker] = useState<{
  propName: string;
  filter: string;
  position: { top: number; left: number };
} | null>(null);
```

- [ ] **Step 2: Create a TokenizedInput component inline (or a helper)**

Since cards manage their own inputs internally, the `/` token picker will be wired at the **add property** level and the **CollapsibleGroup** level (Layout/Typography rows). For cards (Size/Background/Border/Radius/Spacing), we'll defer token binding to a future card-level enhancement — the current card props interface doesn't expose input-level events.

Add a `TokenizedInput` helper inline in VariantEditor:

```typescript
/** Input wrapper that opens token slash picker when user types "/". */
function TokenizedInput({
  value,
  onChange,
  onSlash,
  placeholder,
  style,
}: {
  value: string;
  onChange: (value: string) => void;
  onSlash: () => void;
  placeholder?: string;
  style?: React.CSSProperties;
}) {
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newVal = e.target.value;
      // Detect "/" typed
      if (newVal.endsWith("/") && !value.endsWith("/")) {
        onSlash();
      }
      onChange(newVal);
    },
    [value, onChange, onSlash],
  );

  return (
    <input
      value={value}
      onChange={handleChange}
      style={style}
      placeholder={placeholder}
    />
  );
}
```

- [ ] **Step 3: Wire slash handler to position calculation**

Add a handler for when `/` is detected:

```typescript
const handleSlashTrigger = useCallback(
  (propName: string, inputEl: HTMLInputElement) => {
    const rect = inputEl.getBoundingClientRect();
    setSlashPicker({
      propName,
      filter: "",
      position: {
        top: rect.bottom + 4,
        left: rect.left,
      },
    });
  },
  [],
);
```

- [ ] **Step 4: Update CollapsibleGroup input rows to use TokenizedInput**

In the Layout and Typography groups, replace the plain `<input>` elements with `<TokenizedInput>`. For each plain input in renderCards:

Replace:
```tsx
<input value={value} onChange={(e) => handleSinglePropChange(name, e.target.value)}
  style={INPUT_STYLE} placeholder="value" />
```

With:
```tsx
<TokenizedInput
  value={value}
  onChange={(v) => handleSinglePropChange(name, v)}
  onSlash={() => {
    const el = document.activeElement as HTMLInputElement;
    if (el) handleSlashTrigger(name, el);
  }}
  placeholder="value"
  style={INPUT_STYLE}
/>
```

- [ ] **Step 5: Handle token selection**

```typescript
const handleTokenSelect = useCallback(
  (tokenName: string) => {
    if (!slashPicker) return;
    // Replace the trailing "/" with "token(name) " in the current value
    const currentVal = getPropValue(currentProps, slashPicker.propName);
    const newVal = currentVal.slice(0, -1) + `token(${tokenName}) `;
    handleSinglePropChange(slashPicker.propName, newVal);
    setSlashPicker(null);
  },
  [slashPicker, currentProps, handleSinglePropChange],
);
```

- [ ] **Step 6: Render the slash picker**

Add at the end of the main render (before closing the outermost div):

```tsx
{slashPicker && (
  <TokenSlashPicker
    tokens={tokens}
    filter={slashPicker.filter}
    onSelect={handleTokenSelect}
    onClose={() => setSlashPicker(null)}
    position={slashPicker.position}
  />
)}
```

- [ ] **Step 7: Verify build compiles**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1 | head -30`

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: wire / token slash picker into VariantEditor property inputs"
```

---

### Task 7: Clean up — remove dead code and verify

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Remove old property editing code**

Remove the now-unused functions:
- `renderPropertyRow` (line 739-806)
- `renderPropertyEditor` (line 810-866) — already replaced by `renderCards`
- `renderAddPropertyButton` (line 870-925) — already replaced by `renderAddProperty` in renderCards
- `handlePropertyChange` (line 384-408) — replaced by `handleCardPropsChange`
- `handleRemoveProperty` (line 411-436)
- `handleAddPropertyNameSubmit` (line 438-444)
- `updatePropsList` (line 362-376)
- `removePropFromList` (line 378-382)
- The `addPropRef`, `showAddProp`, `addPropName` state variables (if no longer used by any rendered code)

Also remove the `PROP_ROW`, `PROP_LABEL`, `INHERITED_LABEL`, `REMOVE_BTN`, `DASHED_BTN`, `VALUE_PILL`, `VALUE_PILL_DEFAULT` style constants if they're no longer referenced.

- [ ] **Step 2: Verify no unused imports or variables**

Run: `cd poiesis && pnpm tsc --noEmit 2>&1`
Expected: No TS errors; if there are unused import/variable warnings, clean them up.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "refactor: remove dead code from old VariantEditor property editing"
```

---

### Task 8: Browser verification via Victauri MCP

**Prerequisites:** Poiesis running with Victauri connected.

- [ ] **Step 1: Open Poiesis and navigate to Design tab → Variants**

- [ ] **Step 2: Create a test variant if none exists**

Create a variant named "TestButton" with dimensions:
- `color: primary | danger`
- `size: sm | lg`
Set defaults: `color=primary, size=sm`

- [ ] **Step 3: Verify cards render**

Expected to see: SizeCard, BackgroundCard, BorderCard, RadiusCard, SpacingCard in the property area.

- [ ] **Step 4: Edit base properties via cards**

Change width to 120 in SizeCard, set background to #6366f1 in BackgroundCard.
Expected: Cards update, values persist.

- [ ] **Step 5: Switch to a branch tab (e.g., color.danger)**

Expected: Base properties dimmed (inherited), branch-specific properties editable with blue override dots.

- [ ] **Step 6: Add a branch override**

Set background to #ef4444 in color.danger branch via BackgroundCard.
Expected: BackgroundCard shows red, blue override dot appears.

- [ ] **Step 7: Switch back to base tab**

Expected: Base background is still #6366f1 (danger override didn't affect base).

- [ ] **Step 8: Verify preview panel**

Switch dimension selectors in preview to `color=danger, size=lg`.
Expected: Preview shows red background, size lg dimensions.

- [ ] **Step 9: Test `/` token picker**

In Layout group, type `/` in any text input.
Expected: Token picker popover appears, shows available tokens grouped by category.

- [ ] **Step 10: Select a token from picker**

Click a token from the picker.
Expected: `token(name)` is inserted into the input value.

---

### Task 9: Final commit

- [ ] **Step 1: Final commit with all changes**

```bash
git add .
git commit -m "feat: redesign VariantEditor with card-based properties, / token binding, and live preview"
```
