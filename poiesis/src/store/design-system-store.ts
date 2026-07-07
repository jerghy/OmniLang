// ── Standalone Design System Store ──
// Manages design tokens, classes, variants, and element bindings independently of
// the monolithic PoiesisStore. The monolithic store subscribes to this store for sync.
// IMPORTANT: This store does NOT manage isDirty — the monolithic store handles that.

import { create } from "zustand";
import type {
  DesignDefinition,
  DesignToken,
  DesignClass,
  VariantDefinition,
  ElementVariantBinding,
} from "../core/types/design";
import { EMPTY_DESIGN } from "../core/types/design";
import { uid } from "../core/utils";

// ── Store Interface ──

export interface DesignSystemSlice {
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

// ── Store ──

export const useDesignSystemStore = create<DesignSystemSlice>((set) => ({
  activeDesign: { ...EMPTY_DESIGN },
  elementVariants: {},

  // ── Design lifecycle ──

  loadDesign: (design: DesignDefinition) => set({ activeDesign: design }),

  // ── Tokens ──

  addToken: (name: string, category: string, tokenType: DesignToken["tokenType"], value: string) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: [
          ...s.activeDesign.tokens,
          { id: uid("tk"), name, category, tokenType, value },
        ],
      },
    })),

  updateToken: (id: string, updates: Partial<DesignToken>) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.map((t: DesignToken) =>
          t.id === id ? { ...t, ...updates } : t
        ),
      },
    })),

  removeToken: (id: string) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.filter((t: DesignToken) => t.id !== id),
      },
    })),

  // ── Classes ──

  addDesignClass: (name: string) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: [
          ...s.activeDesign.classes,
          { id: uid("cls"), name, properties: [], pseudoStates: [] },
        ],
      },
    })),

  updateDesignClass: (id: string, updates: Partial<DesignClass>) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.map((c: DesignClass) =>
          c.id === id ? { ...c, ...updates } : c
        ),
      },
    })),

  removeDesignClass: (id: string) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.filter((c: DesignClass) => c.id !== id),
      },
    })),

  // ── Variants ──

  addVariant: (name: string) =>
    set((s) => ({
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
    })),

  updateVariant: (id: string, updates: Partial<VariantDefinition>) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.map((v: VariantDefinition) =>
          v.id === id ? { ...v, ...updates } : v
        ),
      },
    })),

  removeVariant: (id: string) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.filter((v: VariantDefinition) => v.id !== id),
      },
    })),

  // ── Element bindings ──

  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) =>
    set((s) => ({
      elementVariants: {
        ...s.elementVariants,
        [elementId]: { variantName, dimensionValues },
      },
    })),

  unbindVariantFromElement: (elementId: string) =>
    set((s) => {
      const { [elementId]: _, ...rest } = s.elementVariants;
      return { elementVariants: rest };
    }),
}));
