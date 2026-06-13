/** Command: atomically replace all elements and wires.
 *  Used by save/load (.canvas.json) and auto-layout for undo/redo support.
 *
 *  Snapshots the current state on first execute() so undo can restore it. */

import type { Command, CommandContext } from "./types";
import type { ElementData, WireData } from "../store/editor";

export class SetElementsAndWires implements Command {
  readonly type = "SetElementsAndWires";
  readonly description: string;
  private _prevElements: ElementData[] = [];
  private _prevWires: WireData[] = [];
  private readonly _newElements: ElementData[];
  private readonly _newWires: WireData[];
  private _executed = false;

  constructor(
    elements: ElementData[],
    wires: WireData[],
    description = "Replace canvas state",
  ) {
    this._newElements = elements;
    this._newWires = wires;
    this.description = description;
  }

  execute(ctx: CommandContext): void {
    if (!this._executed) {
      // First execution: snapshot the current state for undo
      this._prevElements = [...ctx.elements];
      this._prevWires = [...ctx.wires];
      this._executed = true;
    }
    ctx.setElementsAndWires(this._newElements, this._newWires);
  }

  undo(ctx: CommandContext): void {
    ctx.setElementsAndWires(this._prevElements, this._prevWires);
  }
}
