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
            color: "var(--ol-text-secondary)",
            textTransform: "uppercase",
            marginBottom: 6,
          }}
        >
          Variant 绑定
        </div>

        <div
          style={{
            padding: "6px 8px",
            background: "var(--ol-color-success-subtle)",
            borderRadius: 4,
            border: "1px solid var(--ol-color-success-muted)",
            fontSize: 12,
            color: "var(--ol-color-success-active)",
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
                display: "flex",
                alignItems: "center",
                gap: 6,
                flex: 1,
              }}
            >
              <span
                style={{
                  fontSize: 12,
                  color: "var(--ol-text-secondary)",
                  minWidth: 50,
                  flexShrink: 0,
                }}
              >
                {dim.name}
              </span>
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
                  border: "1px solid var(--ol-border)",
                  borderRadius: 4,
                  outline: "2px solid transparent",
                  background: "var(--ol-surface)",
              }}
            >
              {dim.values.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
            </label>
          </div>
        ))}

        <button type="button"
          onClick={handleUnbind}
          style={{
            marginTop: 6,
            padding: "3px 8px",
            fontSize: 12,
            border: "1px solid var(--ol-color-danger-muted)",
            borderRadius: 4,
            background: "var(--ol-color-danger-subtle)",
            color: "var(--ol-color-danger-hover)",
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
          color: "var(--ol-text-secondary)",
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
            border: "1px solid var(--ol-border)",
            borderRadius: 4,
            outline: "2px solid transparent",
            background: "var(--ol-surface)",
            color: selectedVariant ? "var(--ol-gray-700)" : "var(--ol-text-muted)",
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
            background: selectedVariant ? "var(--ol-color-primary)" : "var(--ol-border)",
            color: selectedVariant ? "var(--ol-surface)" : "var(--ol-text-muted)",
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
