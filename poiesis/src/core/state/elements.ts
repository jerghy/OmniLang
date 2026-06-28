// core/state/elements.ts
// Element CRUD operations — extracted from store/editor.ts v2.

import { create } from "zustand";
import type { ElementData } from "../types/element";

interface ElementState {
  elements: ElementData[];
  rootOrder: string[];

  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  setElementSource: (id: string, source: string) => void;
  setElementPorts: (id: string, ports: ElementData["ports"]) => void;
  setEventBindings: (id: string, eventName: string, binding: {
    fnName: string;
    paramBindings: Record<string, { sourceType: string; value: string; expectedType: string }>;
  }) => void;
  removeEventBindings: (id: string, eventName: string) => void;
  setBlockValue: (id: string, value: unknown) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  setElementId: (oldId: string, newId: string) => void;
  setElementsAndWires: (elements: ElementData[], wires: import("../types/wire").WireData[]) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
}

export const useElementStore = create<ElementState>((set, get) => ({
  elements: [],
  rootOrder: [],

  addElement: (el) =>
    set((s) => {
      const normalized = { ...el, props: el.props || {} };
      const elements = [...s.elements, normalized];
      if (normalized.parentId) {
        return {
          elements: elements.map((e) =>
            e.id === normalized.parentId
              ? { ...e, children: [...e.children, normalized.id] }
              : e
          ),
        };
      }
      return { elements, rootOrder: [...s.rootOrder, normalized.id] };
    }),

  removeElement: (id) =>
    set((s) => ({
      elements: (() => {
        const el = s.elements.find((e) => e.id === id);
        let elements = s.elements.filter((e) => e.id !== id);
        if (el?.parentId) {
          elements = elements.map((e) =>
            e.id === el.parentId
              ? { ...e, children: e.children.filter((cid) => cid !== id) }
              : e
          );
        }
        elements = elements.filter((e) => e.parentId !== id);
        return elements;
      })(),
      rootOrder: s.rootOrder.filter((rid) => rid !== id),
    })),

  setProp: (id, prop, value) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
      ),
    })),

  getProp: (id, prop) => get().elements.find((e) => e.id === id)?.props[prop],

  setElementSource: (id, source) =>
    set((s) => ({ elements: s.elements.map((e) => (e.id === id ? { ...e, source } : e)) })),

  setElementPorts: (id, ports) =>
    set((s) => ({ elements: s.elements.map((e) => (e.id === id ? { ...e, ports } : e)) })),

  setEventBindings: (id, eventName, binding) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        return { ...e, config: { ...e.config, _eventBindings: { ...currentBindings, [eventName]: binding } } };
      }),
    })),

  removeEventBindings: (id, eventName) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        const updated = { ...currentBindings };
        delete updated[eventName];
        return { ...e, config: { ...e.config, _eventBindings: updated } };
      }),
    })),

  setBlockValue: (id, value) =>
    set((s) => {
      const existing = s.elements.find((e) => e.id === id);
      if (existing && JSON.stringify(existing.outputValue) === JSON.stringify(value)) return s;
      return { elements: s.elements.map((e) => (e.id === id ? { ...e, outputValue: value } : e)) };
    }),

  setElementPosition: (id, pos) =>
    set((s) => ({ elements: s.elements.map((e) => (e.id === id ? { ...e, position: { ...pos } } : e)) })),

  setElementId: (oldId, newId) =>
    set((s) => {
      if (s.elements.some((e) => e.id === newId)) return s;
      return {
        elements: s.elements.map((e) => {
          if (e.id === oldId) return { ...e, id: newId };
          if (e.parentId === oldId) return { ...e, parentId: newId };
          return e;
        }),
        rootOrder: s.rootOrder.map((id) => (id === oldId ? newId : id)),
      };
    }),

  setElementsAndWires: (elements, _wires) =>
    set({ elements, rootOrder: elements.filter((e) => !e.parentId).map((e) => e.id) }),

  reorderElement: (elementId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      const target = s.elements.find((e) => e.id === targetSiblingId);
      if (!el || !target) return s;
      if (el.parentId === null || target.parentId === null) {
        if (el.parentId !== target.parentId) return s;
        const order = [...s.rootOrder];
        const fromIdx = order.indexOf(elementId);
        const toIdx = order.indexOf(targetSiblingId);
        if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
        order.splice(fromIdx, 1);
        const newToIdx = order.indexOf(targetSiblingId);
        const insertIdx = position === "before" ? newToIdx : newToIdx + 1;
        order.splice(insertIdx, 0, elementId);
        const elements = [...s.elements];
        const [movedEl] = elements.splice(fromIdx, 1);
        elements.splice(insertIdx, 0, movedEl);
        return { rootOrder: order, elements };
      }
      if (el.parentId !== target.parentId) return s;
      const parentId = el.parentId;
      const parent = s.elements.find((e) => e.id === parentId);
      if (!parent) return s;
      const children = [...parent.children];
      const fromIdx = children.indexOf(elementId);
      const toIdx = children.indexOf(targetSiblingId);
      if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
      children.splice(fromIdx, 1);
      const insertIdx = position === "before" ? children.indexOf(targetSiblingId) : children.indexOf(targetSiblingId) + 1;
      children.splice(insertIdx, 0, elementId);
      return { elements: s.elements.map((e) => (e.id === parentId ? { ...e, children } : e)) };
    }),

  moveElement: (elementId, newParentId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      if (!el) return s;
      const oldParentId = el.parentId;
      let elements = s.elements.map((e) => (e.id === elementId ? { ...e, parentId: newParentId } : e));
      if (oldParentId) {
        elements = elements.map((e) =>
          e.id === oldParentId ? { ...e, children: e.children.filter((cid) => cid !== elementId) } : e
        );
      }
      if (newParentId) {
        const newParent = elements.find((e) => e.id === newParentId);
        if (!newParent) return s;
        const newChildren = [...newParent.children];
        if (targetSiblingId && newChildren.includes(targetSiblingId)) {
          const targetIdx = newChildren.indexOf(targetSiblingId);
          const insertIdx = position === "before" ? targetIdx : targetIdx + 1;
          newChildren.splice(insertIdx, 0, elementId);
        } else {
          newChildren.push(elementId);
        }
        elements = elements.map((e) => (e.id === newParentId ? { ...e, children: newChildren } : e));
      }
      return { elements };
    }),
}));
