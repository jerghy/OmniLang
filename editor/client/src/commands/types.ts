/** Command 模式类型定义。*/

import type { ElementData, WireData } from "../store/editor";

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
}

export interface Command {
  readonly type: string;
  readonly description: string;
  execute(ctx: CommandContext): void;
  undo(ctx: CommandContext): void;
}
