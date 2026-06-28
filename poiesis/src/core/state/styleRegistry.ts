// core/state/styleRegistry.ts
// Atom store — canonical owner of component-level StyleRegistry data.

import { create } from "zustand";
import type { CssVariable, StyleClass, StyleProperty, StyleRegistry } from "../types/style";

/** Stable sentinel used when a component has no registry yet.
 *  Do NOT inline `?? { variables: [], classes: [] }` — that creates a new reference
 *  on every call, which triggers React 18's useSyncExternalStore infinite loop detection. */
const NO_REGISTRY: StyleRegistry = Object.freeze({ variables: [], classes: [] });

interface StyleRegistryState {
  registries: Record<string, StyleRegistry>;

  // ── CSS Variable operations ──
  addVariable: (componentName: string, variable: CssVariable) => void;
  removeVariable: (componentName: string, variableId: string) => void;
  updateVariable: (componentName: string, variableId: string, patch: Partial<CssVariable>) => void;

  // ── Class operations ──
  addClass: (componentName: string, styleClass: StyleClass) => void;
  removeClass: (componentName: string, classId: string) => void;
  replaceClass: (componentName: string, styleClass: StyleClass) => void;
  renameClass: (componentName: string, classId: string, newName: string) => void;

  // ── Property operations ──
  addProperty: (componentName: string, classId: string, property: StyleProperty) => void;
  removeProperty: (componentName: string, classId: string, propertyId: string) => void;
  updateProperty: (componentName: string, classId: string, propertyId: string, patch: Partial<StyleProperty>) => void;

  // ── Queries ──
  getRegistry: (componentName: string) => StyleRegistry;
}

export const useStyleRegistryStore = create<StyleRegistryState>((set, get) => ({
  registries: {},

  addVariable: (componentName, variable) => set((s) => {
    const reg = s.registries[componentName] ?? { variables: [], classes: [] };
    return {
      registries: {
        ...s.registries,
        [componentName]: { ...reg, variables: [...reg.variables, variable] },
      },
    };
  }),

  removeVariable: (componentName, variableId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          variables: reg.variables.filter((v) => v.id !== variableId),
        },
      },
    };
  }),

  updateVariable: (componentName, variableId, patch) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          variables: reg.variables.map((v) =>
            v.id === variableId ? { ...v, ...patch } : v
          ),
        },
      },
    };
  }),

  addClass: (componentName, styleClass) => set((s) => {
    const reg = s.registries[componentName] ?? { variables: [], classes: [] };
    return {
      registries: {
        ...s.registries,
        [componentName]: { ...reg, classes: [...reg.classes, styleClass] },
      },
    };
  }),

  removeClass: (componentName, classId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.filter((c) => c.id !== classId),
        },
      },
    };
  }),

  replaceClass: (componentName, styleClass) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === styleClass.id ? styleClass : c
          ),
        },
      },
    };
  }),

  renameClass: (componentName, classId, newName) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId ? { ...c, name: newName } : c
          ),
        },
      },
    };
  }),

  addProperty: (componentName, classId, property) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? { ...c, properties: [...c.properties, property] }
              : c
          ),
        },
      },
    };
  }),

  removeProperty: (componentName, classId, propertyId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? { ...c, properties: c.properties.filter((p) => p.id !== propertyId) }
              : c
          ),
        },
      },
    };
  }),

  updateProperty: (componentName, classId, propertyId, patch) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? {
                  ...c,
                  properties: c.properties.map((p) =>
                    p.id === propertyId ? { ...p, ...patch } : p
                  ),
                }
              : c
          ),
        },
      },
    };
  }),

  getRegistry: (componentName) => {
    return get().registries[componentName] ?? NO_REGISTRY;
  },
}));
