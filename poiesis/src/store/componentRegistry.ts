/** 自定义组件注册表 — 组件定义存储、统一元数据、多画布支持。
 *
 *  解决 ComponentPalette 和 PropertyPanel 的 META 重复问题：
 *  getUnifiedComponentMeta() 返回内置 + 自定义的合并视图。
 */

import { create } from "zustand";
import type { ElementData, WireData } from "./editor";
import { uid } from "../lib/utils/id";

// ── Schema Types ────────────────────────────────

export interface PropSchema {
  type: string;         // "string" | "enum" | "bool" | "number" | "color"
  default?: unknown;
  values?: string[];    // for enum type
  editor: string;       // "text" | "select" | "checkbox" | "color"
}

export interface EventSchema {
  id: string;
  description: string;
  payload: string | null;
}

export interface ComponentDefinition {
  id: string;
  name: string;
  category: string;
  description: string;

  props: Record<string, PropSchema>;
  events: Record<string, EventSchema>;
  styleProperties: string[];
  children: boolean;

  /** Internal element tree (component canvas state) */
  elements: ElementData[];
  wires: WireData[];

  createdAt: string;
  updatedAt: string;

  /** Storage location preference */
  storage: "config" | "file";
  filePath?: string;

  /** v2: component source marker.
   *  "embedded" = stored inside .omni.json project file
   *  "./path/to/lib.omni-components.json" = loaded from external file
   *  "_external" = lazy-load placeholder, definition not yet loaded */
  source?: string;
}

// ── Registry Store ──────────────────────────────

export interface ComponentRegistryState {
  definitions: Record<string, ComponentDefinition>;

  defineComponent: (name: string, meta: {
    category?: string;
    description?: string;
    props?: Record<string, PropSchema>;
    events?: Record<string, EventSchema>;
    styleProperties?: string[];
    children?: boolean;
    storage?: "config" | "file";
  }) => string;

  removeComponent: (name: string) => void;

  /** Update metadata (props schema, events, etc.) */
  updateMeta: (name: string, patch: Partial<
    Pick<ComponentDefinition, "props" | "events" | "styleProperties" | "children" | "description" | "category" | "storage">
  >) => void;

  /** Save component canvas state */
  saveCanvas: (name: string, elements: ElementData[], wires: WireData[]) => void;

  renameComponent: (oldName: string, newName: string) => void;
}

export const useComponentRegistry = create<ComponentRegistryState>((set) => ({
  definitions: {},

  defineComponent: (name, meta) => {
    const id = uid();
    const now = new Date().toISOString();
    const def: ComponentDefinition = {
      id,
      name,
      category: meta.category || "Custom",
      description: meta.description || "",
      props: meta.props || {},
      events: meta.events || {},
      styleProperties: meta.styleProperties || [],
      children: meta.children ?? true,
      elements: [],
      wires: [],
      createdAt: now,
      updatedAt: now,
      storage: meta.storage || "config",
    };
    set((s) => ({
      definitions: { ...s.definitions, [name]: def },
    }));
    return id;
  },

  removeComponent: (name) =>
    set((s) => {
      const { [name]: _, ...rest } = s.definitions;
      return { definitions: rest };
    }),

  updateMeta: (name, patch) =>
    set((s) => {
      const def = s.definitions[name];
      if (!def) return s;
      return {
        definitions: {
          ...s.definitions,
          [name]: { ...def, ...patch, updatedAt: new Date().toISOString() },
        },
      };
    }),

  saveCanvas: (name, elements, wires) =>
    set((s) => {
      const def = s.definitions[name];
      if (!def) return s;
      return {
        definitions: {
          ...s.definitions,
          [name]: { ...def, elements, wires, updatedAt: new Date().toISOString() },
        },
      };
    }),

  renameComponent: (oldName, newName) =>
    set((s) => {
      const def = s.definitions[oldName];
      if (!def) return s;
      const { [oldName]: _, ...rest } = s.definitions;
      // Update all instance references in the editor store
      const editorStore = window.__ZUSTAND_EDITOR_STORE__;
      if (editorStore) {
        const editorState = editorStore.getState();
        if (editorState?.elements) {
          const updatedElements = editorState.elements.map((el: ElementData) =>
            el.componentRef === oldName ? { ...el, componentRef: newName, tag: newName } : el
          );
          editorStore.setState({ elements: updatedElements });
        }
      }
      return {
        definitions: {
          ...rest,
          [newName]: { ...def, name: newName, updatedAt: new Date().toISOString() },
        },
      };
    }),
}));

// ═══════════════════════════════════════════════
// Unified Metadata — Single Source of Truth
// ═══════════════════════════════════════════════

import { COMPONENT_PROTOCOL } from "../data/componentProtocol";

export interface ComponentMeta {
  component: string;
  category: string;
  description: string;
  props: Record<string, PropSchema>;
  events: Record<string, EventSchema>;
  styleProperties: string[];
  children: boolean;
  isCustom: boolean;
}

/** Returns merged view: built-in protocol + custom definitions.
 *  Custom components override built-in by name. */
export function getUnifiedComponentMeta(): Record<string, ComponentMeta> {
  const builtIn: Record<string, ComponentMeta> = {};
  for (const [name, meta] of Object.entries(COMPONENT_PROTOCOL)) {
    builtIn[name] = {
      component: meta.component,
      category: meta.category,
      description: meta.description,
      props: meta.props as Record<string, PropSchema>,
      events: meta.events as Record<string, EventSchema>,
      styleProperties: meta.styleProperties,
      children: meta.children,
      isCustom: false,
    };
  }

  const custom: Record<string, ComponentMeta> = {};
  for (const def of Object.values(useComponentRegistry.getState().definitions)) {
    custom[def.name] = {
      component: def.name,
      category: def.category,
      description: def.description,
      props: def.props,
      events: def.events,
      styleProperties: def.styleProperties,
      children: def.children,
      isCustom: true,
    };
  }

  return { ...builtIn, ...custom };
}

// Expose registry for E2E tests
if (typeof window !== "undefined") {
  window.__COMPONENT_REGISTRY__ = useComponentRegistry;
}
