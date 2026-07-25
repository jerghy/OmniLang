// poiesis/src/components/canvas/code/blocks/mutations.ts
import * as Blockly from "blockly";

/**
 * The maximum number of elif branches supported.
 */
const MAX_ELIF = 4;

/**
 * Mixin for the if_block mutator.
 *
 * Provides serialization of elif/else branches and dynamic
 * rebuild of the input structure on the block.
 */
export const IF_BLOCK_MUTATOR_MIXIN = {
  /**
   * Serialize the current elif/else config into a <mutation> element.
   */
  mutationToDom(this: Blockly.Block): Element {
    const container = document.createElement("mutation");
    const elifCount = (this as unknown as Record<string, number>).elifCount_ || 0;
    const hasElse = (this as unknown as Record<string, boolean>).hasElse_ || false;
    container.setAttribute("elif", String(elifCount));
    container.setAttribute("else", hasElse ? "1" : "0");
    return container;
  },

  /**
   * Deserialize a <mutation> element and rebuild branch inputs.
   */
  domToMutation(this: Blockly.Block, xmlElement: Element): void {
    const self = this as unknown as MutatorBlock;
    self.elifCount_ = parseInt(xmlElement.getAttribute("elif") || "0", 10);
    self.hasElse_ = xmlElement.getAttribute("else") === "1";
    self.rebuildBranches_();
  },

  /**
   * Rebuild elif/else inputs based on elifCount_ and hasElse_.
   *
   * Preserves existing child blocks wherever possible by
   * disconnecting them before removing inputs and reconnecting
   * them after.
   */
  rebuildBranches_(this: Blockly.Block): void {
    const self = this as unknown as MutatorBlock;

    // Collect existing child blocks before deleting inputs
    const saved: Record<string, Blockly.Block | null> = {};

    // Save the main condition and body
    const conditionBlock = self.getInputTargetBlock("CONDITION");
    const bodyBlock = self.getInputTargetBlock("BODY");

    // Save all elif condition and body blocks
    for (let i = 0; i < MAX_ELIF; i++) {
      const condInput = `ELIF${i}_CONDITION`;
      const bodyInput = `ELIF${i}_BODY`;
      saved[condInput] = self.getInputTargetBlock(condInput);
      saved[bodyInput] = self.getInputTargetBlock(bodyInput);
    }

    // Save else body
    const elseBodyBlock = self.getInputTargetBlock("ELSE_BODY");

    // Remove existing inputs (except CONDITION and BODY which are part of the static def)
    for (let i = 0; i < MAX_ELIF; i++) {
      const condInput = `ELIF${i}_CONDITION`;
      const bodyInput = `ELIF${i}_BODY`;
      if (self.getInput(condInput)) {
        self.removeInput(condInput);
      }
      if (self.getInput(bodyInput)) {
        self.removeInput(bodyInput);
      }
    }

    // Remove the else body input if present (it's built into the static def,
    // but we may want to hide it when hasElse_ is false)
    if (self.getInput("ELSE_BODY")) {
      // We keep ELSE_BODY but clear its connection when hasElse_ is false
    }

    // Re-add elif inputs
    for (let i = 0; i < self.elifCount_; i++) {
      const condInput = `ELIF${i}_CONDITION`;
      const bodyInput = `ELIF${i}_BODY`;

      // Add elif condition input
      self.appendValueInput(condInput)
        .setCheck("Expr")
        .appendField("elif");

      // Add elif body input
      self.appendStatementInput(bodyInput)
        .setCheck("Statement")
        .appendField("do");

      // Reconnect saved child blocks
      const savedCond = saved[condInput];
      if (savedCond) {
        const conn = self.getInput(condInput)?.connection;
        if (conn) {
          conn.connect(savedCond.previousConnection!);
        }
      }

      const savedBody = saved[bodyInput];
      if (savedBody) {
        const conn = self.getInput(bodyInput)?.connection;
        if (conn) {
          conn.connect(savedBody.previousConnection!);
        }
      }
    }

    // Reconnect condition and body (they should still exist from the static def)
    if (conditionBlock) {
      const conn = self.getInput("CONDITION")?.connection;
      if (conn && conditionBlock.previousConnection) {
        conn.connect(conditionBlock.previousConnection);
      }
    }
    if (bodyBlock) {
      const conn = self.getInput("BODY")?.connection;
      if (conn && bodyBlock.previousConnection) {
        conn.connect(bodyBlock.previousConnection);
      }
    }

    // Reconnect else body
    if (elseBodyBlock) {
      if (self.hasElse_) {
        const conn = self.getInput("ELSE_BODY")?.connection;
        if (conn && elseBodyBlock.previousConnection) {
          conn.connect(elseBodyBlock.previousConnection);
        }
      }
    }

    // Show/hide the else section label and input
    if (self.hasElse_) {
      // Ensure the else body input exists and is visible
      // It's part of the static def (message2) so it already exists
    }
  },

  /**
   * Provide a block for the mutator flyout given an element name.
   */
  blockToDom?: (this: Blockly.Block) => Element;

  /**
   * Compose the if_block from the mutator workspace.
   */
  compose(this: Blockly.Block, containerBlock: Blockly.Block): void {
    const self = this as unknown as MutatorBlock;

    // Count elif blocks in the mutator
    let elifCount = 0;
    let child = containerBlock.getInputTargetBlock("STACK");
    while (child) {
      if (child.type === "if_block_elif_case") {
        elifCount++;
      }
      child = child.getNextBlock();
    }

    // Check for else block
    const hasElse = containerBlock.getInputTargetBlock("STACK")?.type === "if_block_else_case";

    self.elifCount_ = elifCount;
    self.hasElse_ = hasElse;
    self.rebuildBranches_();
  },

  /**
   * Decompose the if_block into a mutator workspace.
   */
  decompose(this: Blockly.Block, workspace: Blockly.Workspace): Blockly.Block {
    const self = this as unknown as MutatorBlock;

    // Create the container block
    const containerBlock = workspace.newBlock("if_block_container");
    containerBlock.initSvg();

    // Add elif case blocks
    let connection = containerBlock.getInput("STACK")?.connection;
    for (let i = 0; i < self.elifCount_; i++) {
      const elifBlock = workspace.newBlock("if_block_elif_case");
      elifBlock.initSvg();
      if (connection) {
        connection.connect(elifBlock.previousConnection!);
      }
      connection = elifBlock.nextConnection ?? undefined;
    }

    // Add else case block if needed
    if (self.hasElse_) {
      const elseBlock = workspace.newBlock("if_block_else_case");
      elseBlock.initSvg();
      if (connection) {
        connection.connect(elseBlock.previousConnection!);
      }
    }

    return containerBlock;
  },
};

interface MutatorBlock {
  elifCount_: number;
  hasElse_: boolean;
  rebuildBranches_: () => void;
  getInputTargetBlock: (name: string) => Blockly.Block | null;
  getInput: (name: string) => Blockly.Input | null;
  appendValueInput: (name: string) => Blockly.Input;
  appendStatementInput: (name: string) => Blockly.Input;
  removeInput: (name: string) => void;
}

/**
 * Register the if_block mutator extension and its
 * companion mutator blocks (container, elif_case, else_case).
 */
export function registerIfBlockMutator(): void {
  // Register the mutator extension
  Blockly.Extensions.registerMutator(
    "if_block_mutator",
    IF_BLOCK_MUTATOR_MIXIN,
    undefined as unknown as (() => unknown) | undefined,
  );

  // Register mutator workspace blocks
  if (!Blockly.Blocks["if_block_container"]) {
    Blockly.Blocks["if_block_container"] = {
      init(this: Blockly.Block) {
        this.jsonInit({
          type: "if_block_container",
          message0: "if container %1",
          args0: [
            { type: "input_statement", name: "STACK" },
          ],
          colour: "#FFAB19",
          tooltip: "Container for if/elif/else branches",
        });
      },
    };
  }

  if (!Blockly.Blocks["if_block_elif_case"]) {
    Blockly.Blocks["if_block_elif_case"] = {
      init(this: Blockly.Block) {
        this.jsonInit({
          type: "if_block_elif_case",
          message0: "elif",
          previousStatement: null,
          nextStatement: null,
          colour: "#FFAB19",
          tooltip: "An elif branch",
        });
      },
    };
  }

  if (!Blockly.Blocks["if_block_else_case"]) {
    Blockly.Blocks["if_block_else_case"] = {
      init(this: Blockly.Block) {
        this.jsonInit({
          type: "if_block_else_case",
          message0: "else",
          previousStatement: null,
          nextStatement: null,
          colour: "#FFAB19",
          tooltip: "An else branch",
        });
      },
    };
  }
}
