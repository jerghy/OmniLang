// ui/inspector/sections/StyleSection.tsx
// Simplified: shows applied classes (from registry) + inline styles for the selected element.

import React from "react";
import type { ElementData, StyleProperty } from "../../../core/types";
import type { StyleRegistry } from "../../../core/types/style";
import type { CommandContext } from "../../../core/commands/types";
import { useStyleRegistryStore } from "../../../core/state/styleRegistry";
import { useUndoStore } from "../../../store/undo";
import { SetElementClassRefs } from "../../../core/commands/SetElementClassRefs";
import { SetElementInlineStyles } from "../../../core/commands/SetElementInlineStyles";
import { StylePropertyEditor } from "../../panels/StylePropertyEditor";
import { uid } from "../../../core/utils/id";

/** Stable empty-registry sentinel — never creates a new reference on each render. */
const EMPTY_REGISTRY: StyleRegistry = { variables: [], classes: [] };

interface Props {
  element: ElementData;
  ctx: CommandContext;
  onRecompile: () => void;
}

export function StyleSection({ element: el, ctx, onRecompile }: Props) {
  const undoExec = useUndoStore((s) => s.execute);
  // ⚠️ Do NOT call s.getRegistry("App") as a selector — it returns a new reference
  // every time the key is absent, causing a useSyncExternalStore infinite loop.
  const registry = useStyleRegistryStore((s) => s.registries["App"] ?? EMPTY_REGISTRY);
  const classRefs = el.style?.classRefs ?? [];
  const inlineProperties: StyleProperty[] = el.style?.inlineProperties ?? [];

  const handleAddClassRef = (className: string) => {
    const newRefs = [...classRefs, className];
    undoExec(new SetElementClassRefs(el.id, classRefs, newRefs), ctx);
    onRecompile();
  };

  const handleRemoveClassRef = (className: string) => {
    const newRefs = classRefs.filter((c: string) => c !== className);
    undoExec(new SetElementClassRefs(el.id, classRefs, newRefs), ctx);
    onRecompile();
  };

  const handleAddInline = () => {
    const newProps = [...inlineProperties, { id: uid("p"), name: "padding", segments: [{ type: "text" as const, value: "" }] }];
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  const handleRemoveInline = (index: number) => {
    const newProps = inlineProperties.filter((_, i) => i !== index);
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  const handleChangeInline = (index: number, prop: StyleProperty) => {
    const newProps = [...inlineProperties];
    newProps[index] = prop;
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  // Get unapplied classes (in registry but not on element)
  const unappliedClasses = registry.classes.filter((c) => !classRefs.includes(c.name));

  return (
    <>
      <div style={SECTION_HEADER}>样式</div>

      {/* Applied classes */}
      <div style={{ marginBottom: 8 }}>
        <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>📎 应用的类</div>
        {classRefs.length === 0 && (
          <div style={{ fontSize: 11, color: "#9CA3AF", padding: "4px 0" }}>无</div>
        )}
        {classRefs.map((cn: string) => (
          <div key={cn} style={{ display: "flex", alignItems: "center", fontSize: 12, padding: "2px 0" }}>
            <span style={{ color: "#374151", flex: 1 }}>.{cn}</span>
            <button
              onClick={() => handleRemoveClassRef(cn)}
              style={{ background: "none", border: "none", color: "#DC2626", cursor: "pointer", fontSize: 14 }}
            >
              ×
            </button>
          </div>
        ))}
        {unappliedClasses.length > 0 && (
          <select
            value=""
            onChange={(e) => { if (e.target.value) handleAddClassRef(e.target.value); }}
            style={{
              width: "100%", marginTop: 4, padding: "2px 6px", fontSize: 11,
              border: "1px solid #d1d5db", borderRadius: 4,
            }}
          >
            <option value="">+ 关联 class...</option>
            {unappliedClasses.map((c) => (
              <option key={c.id} value={c.name}>.{c.name}</option>
            ))}
          </select>
        )}
      </div>

      {/* Inline styles */}
      <div>
        <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>⚡ 内联样式</div>
        {inlineProperties.map((prop, i) => (
          <StylePropertyEditor
            key={prop.id}
            property={prop}
            onChange={(p) => handleChangeInline(i, p)}
            onDelete={() => handleRemoveInline(i)}
          />
        ))}
        <button
          onClick={handleAddInline}
          style={{
            background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
            color: "#4F46E5", cursor: "pointer", fontSize: 11,
            padding: "4px 8px", width: "100%", marginTop: 4,
          }}
        >
          + 添加
        </button>
      </div>
    </>
  );
}

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 11, color: "#6b7280", textTransform: "uppercase",
  margin: "12px 0 4px",
};
