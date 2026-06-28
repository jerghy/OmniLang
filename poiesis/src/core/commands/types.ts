/** Command 模式类型定义。*/

import type { ElementData, WireData } from "../types";

export interface CommandContext {
  getElement: (id: string) => ElementData | undefined;
  elements: ElementData[];
  wires: WireData[];
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  /** Replace all elements and wires atomically (for save/load + auto-layout undo/redo). */
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  /** Reorder an element within its parent's children array. */
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  /** Move an element to a different parent container (or to root if newParentId is null). */
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
  /** Get the current rootOrder array (for root-level reorder undo). */
  getRootOrder?: () => string[];
}

/** Context for code graph (FlowCanvas) commands.
 *  Same shape as CommandContext but operates on codeElements/codeWires. */
export interface CodeGraphCommandContext {
  getElement: (id: string) => ElementData | undefined;
  elements: ElementData[];
  wires: WireData[];
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
}

export interface Command {
  readonly type: string;
  readonly description: string;
  execute(ctx: CommandContext | CodeGraphCommandContext): void;
  undo(ctx: CommandContext | CodeGraphCommandContext): void;
}
