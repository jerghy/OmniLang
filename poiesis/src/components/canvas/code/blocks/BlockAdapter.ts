// poiesis/src/components/canvas/code/blocks/BlockAdapter.ts
// Bidirectional converter between CodeOp[] and Blockly blocks.
// Maps 13 operation types (declare_var, set_var, return_stmt, call_fn,
// await_expr, break_stmt, continue_stmt, comment_stmt, raw_stmt,
// if_block, while_loop, for_in) to/from Blockly block instances.

import * as Blockly from "blockly";
import type { CodeOp } from "../../../../core/types/code-op";
import { slotToText, fromLegacyString } from "../../../../core/types/slot-value";
import type { SlotValue } from "../../../../core/types/slot-value";
import { registerAllBlocks } from "./definitions";
import { createBodyId } from "../../../../core/types/code-op";

/** Generate a pseudo-random pid for operations created from blocks. */
function generatePid(): string {
  return "pid-" + Math.random().toString(36).slice(2, 10);
}

/**
 * BlockAdapter — converts CodeOp[] <-> Blockly blocks.
 *
 * Call `toBlocks(ops, workspace)` to render operations as blocks.
 * Call `fromBlocks(workspace)` to read operations back from blocks.
 */
export class BlockAdapter {
  constructor() {
    registerAllBlocks();
    this.registerValueBlock_();
  }

  // ── Public API ──

  /**
   * Convert CodeOp[] into Blockly blocks on the given workspace.
   * Clears the workspace first.
   */
  toBlocks(ops: CodeOp[], workspace: Blockly.Workspace): Blockly.Block[] {
    workspace.clear();
    const blocks: Blockly.Block[] = [];
    for (const op of ops) {
      const block = this.createBlock(op, workspace);
      if (block) blocks.push(block);
    }
    return blocks;
  }

  /**
   * Set a text value on a slot input by creating/replacing a _value child block.
   * Exposed publicly so the BlockEditor can set slot values from fuzzy search
   * selections without knowing the _value block internals.
   */
  setSlotValue(
    block: Blockly.Block,
    inputName: string,
    text: string,
  ): void {
    const input = block.getInput(inputName);
    if (!input) return;

    // Dispose any existing child block
    const existing = input.connection?.targetBlock();
    if (existing && existing.isDeletable()) {
      existing.dispose(true);
    }

    const child = block.workspace.newBlock("_value");
    child.setFieldValue(text, "TEXT");
    const conn = input.connection;
    if (conn && child.outputConnection) {
      conn.connect(child.outputConnection);
    }
  }

  /**
   * Convert the top-level blocks on a workspace back into CodeOp[].
   */
  fromBlocks(workspace: Blockly.Workspace): CodeOp[] {
    const ops: CodeOp[] = [];
    const topBlocks = workspace.getTopBlocks(true);
    for (const block of topBlocks) {
      const op = this.blockToCodeOp(block);
      if (op) ops.push(op);
    }
    return ops;
  }

  // ── Block construction (toBlocks) ──

  private registerValueBlock_(): void {
    if (Blockly.Blocks["_value"]) return;
    Blockly.Blocks["_value"] = {
      init(this: Blockly.Block) {
        this.jsonInit({
          type: "_value",
          message0: "%1",
          args0: [{ type: "field_input", name: "TEXT", text: "" }],
          output: null,
          colour: "#ffffff",
        } as unknown as Blockly.BlockDefinition);
      },
    };
  }

  private createBlock(op: CodeOp, ws: Blockly.Workspace): Blockly.Block | null {
    let block: Blockly.Block | null = null;

    switch (op.kind) {
      case "declare_var": {
        block = ws.newBlock("declare_var");
        block.setFieldValue(op.mutable ? "mut" : "let", "MUT");
        this.setSlotInInput(block, "NAME", op.name);
        this.setSlotInInput(block, "VARTYPE", op.varType ?? { kind: "empty" });
        this.setSlotInInput(block, "VALUE", op.initValue ?? { kind: "empty" });
        break;
      }
      case "set_var": {
        block = ws.newBlock("set_var");
        const opLabel = op.operator === "=" ? "set" : op.operator;
        block.setFieldValue(opLabel, "OP");
        this.setSlotInInput(block, "NAME", op.name);
        this.setSlotInInput(block, "VALUE", op.value);
        break;
      }
      case "return": {
        block = ws.newBlock("return_stmt");
        if (op.value) {
          this.setSlotInInput(block, "VALUE", op.value);
        }
        break;
      }
      case "call_fn": {
        block = ws.newBlock("call_fn");
        this.setSlotInInput(block, "FN_NAME", op.fnName);
        if (op.args.length > 0) {
          this.setSlotInInput(block, "ARGS", op.args[0]);
        }
        break;
      }
      case "await": {
        block = ws.newBlock("await_expr");
        this.setSlotInInput(block, "VALUE", op.value);
        break;
      }
      case "break": {
        block = ws.newBlock("break_stmt");
        break;
      }
      case "continue": {
        block = ws.newBlock("continue_stmt");
        break;
      }
      case "comment": {
        block = ws.newBlock("comment_stmt");
        block.setFieldValue(op.text, "TEXT");
        break;
      }
      case "raw": {
        block = ws.newBlock("raw_stmt");
        block.setFieldValue(op.source, "SOURCE");
        break;
      }
      case "if_block": {
        block = ws.newBlock("if_block");
        const branch0 = op.branches[0];
        if (branch0) {
          this.setSlotInInput(block, "CONDITION", branch0.condition);
          this.createBodyBlocks(block, "BODY", branch0.body, ws);
        }

        // Count elif branches (exclude if and else branches)
        const hasElse =
          op.branches.length > 1 &&
          op.branches[op.branches.length - 1].condition.kind === "empty";
        const elifCount = op.branches.length - 1 - (hasElse ? 1 : 0);

        // Apply mutation: create elif/else inputs via domToMutation
        const mutationEl = document.createElement("mutation");
        mutationEl.setAttribute("elif", String(elifCount));
        mutationEl.setAttribute("else", hasElse ? "1" : "0");
        block.domToMutation(mutationEl);

        // Override mutationToDom to capture branch counts from this op
        block.mutationToDom = () => {
          const m = document.createElement("mutation");
          m.setAttribute("elif", String(elifCount));
          m.setAttribute("else", hasElse ? "1" : "0");
          return m;
        };

        // Fill elif/else branches
        for (let i = 1; i < op.branches.length; i++) {
          const b = op.branches[i];
          const isElse =
            i === op.branches.length - 1 && b.condition.kind === "empty";
          if (!isElse) {
            // Mutator naming: ELIF0_CONDITION, ELIF1_CONDITION, ...
            this.setSlotInInput(block, `ELIF${i - 1}_CONDITION`, b.condition);
          }
          const bodyName = isElse ? "ELSE_BODY" : `ELIF${i - 1}_BODY`;
          this.createBodyBlocks(block, bodyName, b.body, ws);
        }
        break;
      }
      case "while": {
        block = ws.newBlock("while_loop");
        this.setSlotInInput(block, "CONDITION", op.condition);
        this.createBodyBlocks(block, "BODY", op.body, ws);
        break;
      }
      case "for_in": {
        block = ws.newBlock("for_in");
        this.setSlotInInput(block, "ITEM", op.itemName);
        this.setSlotInInput(block, "COLLECTION", op.collection);
        this.createBodyBlocks(block, "BODY", op.body, ws);
        break;
      }
    }

    return block;
  }

  /**
   * Set a slot value on an input_value by creating a child _value block
   * and connecting it to the input's connection.
   */
  private setSlotInInput(
    block: Blockly.Block,
    inputName: string,
    sv: SlotValue,
  ): void {
    const input = block.getInput(inputName);
    if (!input) return;
    const text = slotToText(sv);
    if (!text) return;

    const child = block.workspace.newBlock("_value");
    child.setFieldValue(text, "TEXT");

    const conn = input.connection;
    if (conn && child.outputConnection) {
      conn.connect(child.outputConnection);
    }
  }

  /**
   * Create a chain of statement blocks attached to a statement input.
   */
  private createBodyBlocks(
    parent: Blockly.Block,
    inputName: string,
    body: CodeOp[],
    ws: Blockly.Workspace,
  ): void {
    const input = parent.getInput(inputName);
    if (!input || body.length === 0) return;
    const conn = input.connection;
    if (!conn) return;

    let prev: Blockly.Block | null = null;
    for (const op of body) {
      const child = this.createBlock(op, ws);
      if (!child) continue;

      if (!prev) {
        // First block: connect to statement input
        if (child.previousConnection) {
          conn.connect(child.previousConnection);
        }
      } else {
        // Subsequent block: chain via nextConnection
        if (prev.nextConnection && child.previousConnection) {
          prev.nextConnection.connect(child.previousConnection);
        }
      }
      prev = child;
    }
  }

  // ── Block reading (fromBlocks) ──

  private blockToCodeOp(block: Blockly.Block): CodeOp | null {
    const pid = generatePid();

    switch (block.type) {
      case "declare_var": {
        const varType = this.getSlotValue(block, "VARTYPE");
        const initValue = this.getSlotValue(block, "VALUE");
        return {
          kind: "declare_var",
          pid,
          mutable: block.getFieldValue("MUT") === "mut",
          name: this.getSlotValue(block, "NAME"),
          ...(varType.kind !== "empty" ? { varType } : {}),
          ...(initValue.kind !== "empty" ? { initValue } : {}),
          scope: "fn",
        };
      }
      case "set_var": {
        const opVal = block.getFieldValue("OP") || "set";
        const opMap: Record<string, "=" | "+=" | "-=" | "*=" | "/=" | "%="> = {
          set: "=",
          "+=": "+=",
          "-=": "-=",
          "*=": "*=",
          "/=": "/=",
          "%=": "%=",
        };
        return {
          kind: "set_var",
          pid,
          name: this.getSlotValue(block, "NAME"),
          operator: opMap[opVal] || "=",
          value: this.getSlotValue(block, "VALUE"),
        };
      }
      case "return_stmt": {
        const v = this.getSlotValue(block, "VALUE");
        return {
          kind: "return",
          pid,
          ...(v.kind !== "empty" ? { value: v } : {}),
        };
      }
      case "call_fn": {
        const arg = this.getSlotValue(block, "ARGS");
        return {
          kind: "call_fn",
          pid,
          fnName: this.getSlotValue(block, "FN_NAME"),
          args: arg.kind !== "empty" ? [arg] : [],
        };
      }
      case "await_expr": {
        return {
          kind: "await",
          pid,
          value: this.getSlotValue(block, "VALUE"),
        };
      }
      case "break_stmt": {
        return { kind: "break", pid };
      }
      case "continue_stmt": {
        return { kind: "continue", pid };
      }
      case "comment_stmt": {
        return {
          kind: "comment",
          pid,
          text: block.getFieldValue("TEXT") || "",
        };
      }
      case "raw_stmt": {
        return {
          kind: "raw",
          pid,
          source: block.getFieldValue("SOURCE") || "",
        };
      }
      case "if_block": {
        const condition = this.getSlotValue(block, "CONDITION");
        const body = this.getStatementBody(block, "BODY");

        // Read mutation state from the block instance
        const elifCount = (block as unknown as Record<string, number>)
          .elifCount_ || 0;
        const hasElse = !!(
          block as unknown as Record<string, boolean>
        ).hasElse_;

        const branches: { condition: SlotValue; body: CodeOp[] }[] = [
          { condition, body },
        ];

        // Read elif branches
        for (let i = 0; i < elifCount; i++) {
          const elifCond = this.getSlotValue(
            block,
            `ELIF${i}_CONDITION`,
          );
          const elifBody = this.getStatementBody(block, `ELIF${i}_BODY`);
          branches.push({ condition: elifCond, body: elifBody });
        }

        // Read else branch
        if (hasElse) {
          const elseBody = this.getStatementBody(block, "ELSE_BODY");
          branches.push({
            condition: { kind: "empty" } as SlotValue,
            body: elseBody,
          });
        }

        return {
          kind: "if_block",
          pid,
          bodyId: createBodyId(),
          branches,
        };
      }
      case "while_loop": {
        return {
          kind: "while",
          pid,
          condition: this.getSlotValue(block, "CONDITION"),
          body: this.getStatementBody(block, "BODY"),
          bodyId: createBodyId(),
        };
      }
      case "for_in": {
        return {
          kind: "for_in",
          pid,
          itemName: this.getSlotValue(block, "ITEM"),
          collection: this.getSlotValue(block, "COLLECTION"),
          body: this.getStatementBody(block, "BODY"),
          bodyId: createBodyId(),
        };
      }
      default:
        return null;
    }
  }

  /**
   * Read a slot value from an input_value child block.
   * Returns { kind: "empty" } when no child block is connected.
   */
  private getSlotValue(block: Blockly.Block, inputName: string): SlotValue {
    const target = block.getInputTargetBlock(inputName);
    if (!target) return { kind: "empty" };
    const fieldValue = target.getFieldValue("TEXT");
    if (!fieldValue) return { kind: "empty" };
    return { kind: "text", value: fieldValue };
  }

  /**
   * Read the chain of statement blocks attached to a statement input.
   */
  private getStatementBody(
    block: Blockly.Block,
    inputName: string,
  ): CodeOp[] {
    const input = block.getInput(inputName);
    if (!input) return [];
    const conn = input.connection;
    if (!conn) return [];

    const body: CodeOp[] = [];
    let child = conn.targetBlock();
    while (child) {
      const op = this.blockToCodeOp(child);
      if (op) body.push(op);
      child = child.getNextBlock();
    }
    return body;
  }
}
