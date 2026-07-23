// components/panels/inspector/VariantBindingSection.tsx
// Variant binding section: shows bound variant + dimension dropdowns, or a variant selector.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";

interface VariantBindingSectionProps {
  element: ElementData;
}

export function VariantBindingSection({ element }: VariantBindingSectionProps) {
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const elementVariants = usePoiesisStore((s) => s.elementVariants);
  const bindVariantToElement = usePoiesisStore((s) => s.bindVariantToElement);
  const unbindVariantFromElement = usePoiesisStore(
    (s) => s.unbindVariantFromElement,
  );

  const boundVariant = elementVariants[element.id];
  const [selectedVariant, setSelectedVariant] = useState("");

  const handleBindVariant = useCallback(() => {
    if (!selectedVariant) return;
    const variant = activeDesign.variants.find(
      (v) => v.name === selectedVariant,
    );
    if (!variant) return;
    const defaults: Record<string, string> = {};
    for (const dim of variant.dimensions) {
      defaults[dim.name] = dim.values[0] ?? "";
    }
    bindVariantToElement(element.id, selectedVariant, defaults);
  }, [selectedVariant, activeDesign.variants, element.id, bindVariantToElement]);

  const handleDimensionChange = useCallback(
    (dimName: string, value: string) => {
      if (!boundVariant) return;
      bindVariantToElement(element.id, boundVariant.variantName, {
        ...boundVariant.dimensionValues,
        [dimName]: value,
      });
    },
    [boundVariant, element.id, bindVariantToElement],
  );

  const handleUnbind = useCallback(() => {
    unbindVariantFromElement(element.id);
  }, [element.id, unbindVariantFromElement]);

  const boundVariantDef = boundVariant
    ? activeDesign.variants.find((v) => v.name === boundVariant.variantName)
    : undefined;

  if (boundVariant && boundVariantDef) {
    return (
      <div style={{ marginBottom: 12 }}>
        <div
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: "#6b7280",
            textTransform: "uppercase",
            marginBottom: 6,
          }}
        >
          Variant 绑定
        </div>

        <div
          style={{
            padding: "6px 8px",
            background: "#f0fdf4",
            borderRadius: 4,
            border: "1px solid #bbf7d0",
            fontSize: 12,
            color: "#166534",
            marginBottom: 6,
            fontWeight: 500,
          }}
        >
          {boundVariant.variantName}
        </div>

        {/* Dimension selectors */}
        {boundVariantDef.dimensions.map((dim) => (
          <div
            key={dim.name}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              marginBottom: 4,
            }}
          >
            <label
              style={{
                fontSize: 12,
                color: "#64748b",
                minWidth: 50,
                flexShrink: 0,
              }}
            >
              {dim.name}
            </label>
            <select
              aria-label={dim.name}
              value={
                boundVariant.dimensionValues[dim.name] ?? dim.values[0] ?? ""
              }
              onChange={(e) => handleDimensionChange(dim.name, e.target.value)}
              style={{
                flex: 1,
                padding: "3px 6px",
                fontSize: 12,
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                outline: "2px solid transparent",
                background: "#fff",
              }}
            >
              {dim.values.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </div>
        ))}

        <button type="button"
          onClick={handleUnbind}
          style={{
            marginTop: 6,
            padding: "3px 8px",
            fontSize: 12,
            border: "1px solid #fca5a5",
            borderRadius: 4,
            background: "#fef2f2",
            color: "#dc2626",
            cursor: "pointer",
            fontWeight: 500,
          }}
        >
          解除 variant
        </button>
      </div>
    );
  }

  return (
    <div style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
        }}
      >
        Variant 绑定
      </div>

      <div style={{ display: "flex", gap: 4 }}>
        <select
          aria-label="Select variant"
          value={selectedVariant}
          onChange={(e) => setSelectedVariant(e.target.value)}
          style={{
            flex: 1,
            padding: "3px 6px",
            fontSize: 12,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "2px solid transparent",
            background: "#fff",
            color: selectedVariant ? "#374151" : "#94a3b8",
          }}
        >
          <option value="">选择 variant...</option>
          {activeDesign.variants.map((v) => (
            <option key={v.id} value={v.name}>
              {v.name}
            </option>
          ))}
        </select>
        <button type="button"
          onClick={handleBindVariant}
          disabled={!selectedVariant}
          style={{
            padding: "3px 10px",
            fontSize: 12,
            border: "none",
            borderRadius: 4,
            background: selectedVariant ? "#6366F1" : "#e2e8f0",
            color: selectedVariant ? "#fff" : "#94a3b8",
            cursor: selectedVariant ? "pointer" : "default",
            fontWeight: 600,
          }}
        >
          +
        </button>
      </div>
    </div>
  );
}
