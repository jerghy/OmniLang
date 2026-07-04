// hooks/useOmniViewRender.ts
// Hook for WASM style resolution (placeholder — will use WASM in future).
// Currently resolves styles via the store's inline properties.

import { useMemo } from "react";
import { usePoiesisStore } from "../store";
import type { ElementData } from "../core/types/element";

export interface ResolvedStyle {
  className: string;
  inlineStyles: Record<string, string>;
}

export function useOmniViewRender(elementIds: string[]) {
  const elements = usePoiesisStore((s) => s.elements);

  const resolvedStyles = useMemo(() => {
    const map = new Map<string, ResolvedStyle>();

    for (const id of elementIds) {
      const el = elements.find((e) => e.id === id);
      if (!el) continue;

      const inlineStyles: Record<string, string> = {};

      if (el.style?.inlineProperties) {
        for (const prop of el.style.inlineProperties) {
          if (
            prop.segments.length > 0 &&
            prop.segments[0].type === "text"
          ) {
            inlineStyles[prop.name] = prop.segments[0].value;
          }
        }
      }

      map.set(id, {
        className: el.componentRef ? `cmp-${el.componentRef}` : "",
        inlineStyles,
      });
    }

    return map;
  }, [elements, elementIds]);

  const getStyle = (el: ElementData): React.CSSProperties => {
    const resolved = resolvedStyles.get(el.id);
    if (!resolved) return {};
    return resolved.inlineStyles as React.CSSProperties;
  };

  return { resolvedStyles, getStyle };
}
