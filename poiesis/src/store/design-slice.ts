// poiesis/src/store/design-slice.ts
import type {
  DesignDefinition,
  DesignToken,
  DesignClass,
  VariantDefinition,
  ElementVariantBinding,
} from "../core/types/design";
import { EMPTY_DESIGN } from "../core/types/design";
import { uid } from "../core/utils";
import type { PoiesisState } from "./index";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;

export interface DesignSlice {
  // ── State ──
  activeDesign: DesignDefinition;
  elementVariants: Record<string, ElementVariantBinding>;

  // ── Token actions ──
  loadDesign: (design: DesignDefinition) => void;
  addToken: (name: string, category: string, tokenType: DesignToken["tokenType"], value: string) => void;
  updateToken: (id: string, updates: Partial<DesignToken>) => void;
  removeToken: (id: string) => void;

  // ── Class actions ──
  addDesignClass: (name: string) => void;
  updateDesignClass: (id: string, updates: Partial<DesignClass>) => void;
  removeDesignClass: (id: string) => void;

  // ── Variant actions ──
  addVariant: (name: string) => void;
  updateVariant: (id: string, updates: Partial<VariantDefinition>) => void;
  removeVariant: (id: string) => void;

  // ── Element binding actions ──
  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) => void;
  unbindVariantFromElement: (elementId: string) => void;
}

export const createDesignSlice = (set: SetFn): DesignSlice => ({
  activeDesign: { ...EMPTY_DESIGN },
  elementVariants: {},

  loadDesign: (design: DesignDefinition) => set({ activeDesign: design, isDirty: true }),

  addToken: (name: string, category: string, tokenType: DesignToken["tokenType"], value: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: [
          ...s.activeDesign.tokens,
          { id: uid("tk"), name, category, tokenType, value },
        ],
      },
      isDirty: true,
    })),

  updateToken: (id: string, updates: Partial<DesignToken>) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.map((t: DesignToken) =>
          t.id === id ? { ...t, ...updates } : t
        ),
      },
      isDirty: true,
    })),

  removeToken: (id: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.filter((t: DesignToken) => t.id !== id),
      },
      isDirty: true,
    })),

  addDesignClass: (name: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: [
          ...s.activeDesign.classes,
          { id: uid("cls"), name, properties: [], pseudoStates: [] },
        ],
      },
      isDirty: true,
    })),

  updateDesignClass: (id: string, updates: Partial<DesignClass>) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.map((c: DesignClass) =>
          c.id === id ? { ...c, ...updates } : c
        ),
      },
      isDirty: true,
    })),

  removeDesignClass: (id: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.filter((c: DesignClass) => c.id !== id),
      },
      isDirty: true,
    })),

  addVariant: (name: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: [
          ...s.activeDesign.variants,
          {
            id: uid("var"),
            name,
            dimensions: [],
            defaults: {},
            baseProperties: [],
            branches: [],
          },
        ],
      },
      isDirty: true,
    })),

  updateVariant: (id: string, updates: Partial<VariantDefinition>) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.map((v: VariantDefinition) =>
          v.id === id ? { ...v, ...updates } : v
        ),
      },
      isDirty: true,
    })),

  removeVariant: (id: string) =>
    set((s: PoiesisState) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.filter((v: VariantDefinition) => v.id !== id),
      },
      isDirty: true,
    })),

  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) =>
    set((s: PoiesisState) => ({
      elementVariants: {
        ...s.elementVariants,
        [elementId]: { variantName, dimensionValues },
      },
      isDirty: true,
    })),

  unbindVariantFromElement: (elementId: string) =>
    set((s: PoiesisState) => {
      const { [elementId]: _, ...rest } = s.elementVariants;
      return { elementVariants: rest, isDirty: true };
    }),
});
