// Legacy style registry stub — merged into unified store.
import { usePoiesisStore } from "../../store";
import type { PoiesisState } from "../../store";
import type { StyleRegistry, StyleClass, CssVariable, StyleProperty } from "../../core/types/style";

export const styleRegistry: { classes: Record<string, unknown>; cssVars: Record<string, unknown> } = {
  classes: {},
  cssVars: {},
};

/** Extended store state with legacy style registry methods. */
interface StyleRegistryState extends PoiesisState {
  addClass(componentName: string, styleClass: StyleClass): void;
  removeClass(componentName: string, classId: string): void;
  addVariable(componentName: string, variable: CssVariable): void;
  removeVariable(componentName: string, variableId: string): void;
  addProperty(componentName: string, classId: string, property: StyleProperty): void;
  removeProperty(componentName: string, classId: string, propertyId: string): void;
  renameClass(componentName: string, classId: string, newName: string): void;
  updateProperty(componentName: string, classId: string, propertyId: string, newPatch: Partial<StyleProperty>): void;
  replaceClass(componentName: string, newClass: StyleClass): void;
  updateVariable(componentName: string, variableId: string, newPatch: Partial<CssVariable>): void;
  getRegistry(componentName: string): StyleRegistry;
}

/** Cache for the wrapper functions — created once to avoid allocations. */
const _styleMethods: Pick<
  StyleRegistryState,
  | "addClass"
  | "removeClass"
  | "addVariable"
  | "removeVariable"
  | "addProperty"
  | "removeProperty"
  | "renameClass"
  | "updateProperty"
  | "replaceClass"
  | "updateVariable"
  | "getRegistry"
> = {
  addClass: (componentName, styleClass) => {
    usePoiesisStore.getState().addClass(componentName, styleClass);
  },
  removeClass: (componentName, classId) => {
    usePoiesisStore.getState().removeClass(componentName, classId);
  },
  addVariable: (componentName, variable) => {
    usePoiesisStore.getState().addVariable(componentName, variable);
  },
  removeVariable: (componentName, variableId) => {
    usePoiesisStore.getState().removeVariable(componentName, variableId);
  },
  addProperty: (componentName, classId, property) => {
    usePoiesisStore.getState().addProperty(componentName, classId, property);
  },
  removeProperty: (componentName, classId, propertyId) => {
    usePoiesisStore.getState().removeProperty(componentName, classId, propertyId);
  },
  renameClass: (componentName, classId, newName) => {
    usePoiesisStore.getState().renameClass(componentName, classId, newName);
  },
  updateProperty: (componentName, classId, propertyId, newPatch) => {
    usePoiesisStore.getState().updateProperty(componentName, classId, propertyId, newPatch);
  },
  replaceClass: (componentName, newClass) => {
    usePoiesisStore.getState().replaceClass(componentName, newClass);
  },
  updateVariable: (componentName, variableId, newPatch) => {
    usePoiesisStore.getState().updateVariable(componentName, variableId, newPatch);
  },
  getRegistry: (componentName) => {
    return usePoiesisStore.getState().getRegistry(componentName);
  },
};

/** Build an augmented state with style registry methods. */
function _getState(): StyleRegistryState {
  const base = usePoiesisStore.getState();
  return Object.assign<PoiesisState, typeof _styleMethods>(base, _styleMethods);
}

/** Style registry store shape. */
interface StyleStore {
  (selector?: (state: StyleRegistryState) => unknown): unknown;
  getState(): StyleRegistryState;
  setState(partial: Partial<PoiesisState>): void;
  subscribe(selector?: (s: PoiesisState) => unknown): () => void;
  destroy(): void;
  getRegistry(componentName: string): StyleRegistry;
}

/** Build the hybrid store object (callable + properties). */
function createStyleStore(): StyleStore {
  const store: StyleStore = (selector?: (state: StyleRegistryState) => unknown): unknown => {
    const s = _getState();
    return selector ? selector(s) : s;
  };

  store.getState = _getState;
  store.setState = (partial: Partial<PoiesisState>) => usePoiesisStore.setState(partial);
  store.subscribe = (selector?: (s: PoiesisState) => unknown): (() => void) => {
    return usePoiesisStore.subscribe((s) => selector?.(s));
  };
  store.destroy = () => {};
  store.getRegistry = (componentName: string): StyleRegistry => {
    return usePoiesisStore.getState().getRegistry(componentName);
  };

  return store;
}

export const useStyleRegistryStore = createStyleStore();
