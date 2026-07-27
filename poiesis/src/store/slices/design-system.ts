// ── Design system slice for Poiesis store ──

import type { PoiesisState } from "../types";
import { uid } from "../../core/utils/id";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createDesignSystemSlice(set: SetFn, _get?: GetFn) {
  return {
    loadDesign: (design: import("../../core/types/design").DesignDefinition) => set({ activeDesign: design, isDirty: true }),

    addToken: (name: string, category: string, tokenType: import("../../core/types/design").DesignToken["tokenType"], value: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: [...s.activeDesign.tokens, { id: uid("tk"), name, category, tokenType, value }],
      },
      isDirty: true,
    })),

    updateToken: (id: string, updates: Partial<import("../../core/types/design").DesignToken>) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.map((t) =>
          t.id === id ? { ...t, ...updates } : t
        ),
      },
      isDirty: true,
    })),

    removeToken: (id: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.filter((t) => t.id !== id),
      },
      isDirty: true,
    })),

    addDesignClass: (name: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: [...s.activeDesign.classes, { id: uid("cls"), name, properties: [], pseudoStates: [] }],
      },
      isDirty: true,
    })),

    updateDesignClass: (id: string, updates: Partial<import("../../core/types/design").DesignClass>) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.map((c) =>
          c.id === id ? { ...c, ...updates } : c
        ),
      },
      isDirty: true,
    })),

    removeDesignClass: (id: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.filter((c) => c.id !== id),
      },
      isDirty: true,
    })),

    addVariant: (name: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: [
          ...s.activeDesign.variants,
          { id: uid("var"), name, dimensions: [], defaults: {}, baseProperties: [], branches: [] },
        ],
      },
      isDirty: true,
    })),

    updateVariant: (id: string, updates: Partial<import("../../core/types/design").VariantDefinition>) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.map((v) =>
          v.id === id ? { ...v, ...updates } : v
        ),
      },
      isDirty: true,
    })),

    removeVariant: (id: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.filter((v) => v.id !== id),
      },
      isDirty: true,
    })),

    toggleInheritedProp: (variantId: string, propName: string) => set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.map((v) => {
          if (v.id !== variantId) return v;
          const inherited = v.inheritedProps ?? [];
          const idx = inherited.indexOf(propName);
          const next = idx >= 0
            ? inherited.filter((_, i) => i !== idx)
            : [...inherited, propName];
          return { ...v, inheritedProps: next };
        }),
      },
      isDirty: true,
    })),

    bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) => set((s) => {
      // Sync variantRef.params on the element so source emitter picks up dimension changes
      const elements = s.elements.map((el) =>
        el.id === elementId && "variantRef" in el && el.variantRef
          ? { ...el, variantRef: { ...(el.variantRef as NonNullable<typeof el.variantRef>), params: { ...dimensionValues } } }
          : el,
      );
      return {
        elements,
        elementVariants: { ...s.elementVariants, [elementId]: { variantName, dimensionValues } },
        isDirty: true,
      };
    }),

    unbindVariantFromElement: (elementId: string) => set((s) => {
      const { [elementId]: _, ...rest } = s.elementVariants;
      return { elementVariants: rest, isDirty: true };
    }),
  };
}
